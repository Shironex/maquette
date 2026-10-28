import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useCreditsStore } from '@/state/use-credits-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { CreditsSheet } from '.';

afterEach(() => {
  clearMocks();
  useCreditsStore.setState({
    open: false,
    ledger: null,
    balances: null,
    filter: { project: null, service: null, source: 'api' },
  });
});

async function openSheet(inbox?: string) {
  await openMockWorkspace('configured', undefined, undefined, inbox);
  render(<CreditsSheet />);
  useCreditsStore.getState().openCredits();
  return screen.findByRole('dialog', { name: 'Credits' });
}

describe('the Credits sheet', () => {
  it('shows each balance, the totals and every paid send', async () => {
    const sheet = await openSheet('redo');
    expect(await within(sheet).findByText('1,240')).toBeInTheDocument();
    expect(within(sheet).getByRole('group', { name: 'Meshy balance' })).toHaveTextContent('380');
    expect(within(sheet).getByText('Last 7 days')).toBeInTheDocument();
    expect(within(sheet).getByText('Redo share')).toBeInTheDocument();
    expect(await within(sheet).findAllByRole('listitem')).not.toHaveLength(0);
    expect(sheet).toHaveTextContent('1 line of the ledger could not be read');
  });

  it('renders only the rows in view of a long ledger', async () => {
    const sheet = await openSheet();
    await within(sheet).findAllByRole('listitem');
    const ledger = useCreditsStore.getState().ledger!;
    const one = ledger.spends[0]!;
    const spends = Array.from({ length: 1000 }, (_, i) => ({ ...one, job: `copy-${i}` }));
    act(() => useCreditsStore.setState({ ledger: { ...ledger, spends } }));
    expect(within(sheet).getAllByRole('listitem').length).toBeLessThan(60);
  });

  it('filters by provider and saves what it shows', async () => {
    const sheet = await openSheet();
    await within(sheet).findAllByRole('listitem');
    await userEvent.click(
      within(within(sheet).getByRole('group', { name: 'Provider' })).getByRole('button', {
        name: 'Meshy',
      })
    );
    const rows = within(sheet).getAllByRole('listitem');
    expect(rows.every(r => r.textContent?.includes('Meshy'))).toBe(true);
    await userEvent.click(within(sheet).getByRole('button', { name: 'Export CSV' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice).toMatchObject({
        title: 'Credits saved as CSV',
        detail: expect.stringMatching(/^\d+ sends in ~\/Downloads\/maquette-credits-/),
      })
    );
  });

  it('keeps By hand subscription sends apart from API spend', async () => {
    const sheet = await openSheet();
    await within(sheet).findAllByRole('listitem');
    const api = within(sheet).getAllByRole('listitem').length;
    await userEvent.click(
      within(within(sheet).getByRole('group', { name: 'Paid with' })).getByRole('button', {
        name: 'Subscription',
      })
    );
    const rows = within(sheet).getAllByRole('listitem');
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.length).not.toBe(api);
    expect(rows.every(r => r.textContent?.includes('By hand'))).toBe(true);
    expect(sheet).toHaveTextContent('Subscription credits are estimates');
  });
});
