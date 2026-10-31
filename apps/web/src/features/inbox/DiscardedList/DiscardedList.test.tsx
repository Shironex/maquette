import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DiscardedList } from '.';
import { timeLeft } from './DiscardedList';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'clear');
  await useInboxStore.getState().load();
  await useTriageStore.getState().loadDiscarded();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
  useTriageStore.setState({ discarded: [] });
});

describe('the recently discarded list', () => {
  it('lists each discarded model with why it left, and restores one', async () => {
    render(<DiscardedList />);
    const list = screen.getByRole('region', { name: 'Recently discarded' });
    expect(within(list).getAllByRole('listitem')).toHaveLength(3);
    expect(list).toHaveTextContent('replaced by v2');
    await userEvent.click(screen.getByRole('button', { name: 'Restore Crystal lantern' }));
    await screen.findByText('2 kept 30 days');
    expect(useInboxStore.getState().snapshot!.items.map(i => i.id)).toEqual([
      'crystal_lantern_v1.glb',
    ]);
  });

  it('shows how long each has left', () => {
    render(<DiscardedList />);
    const list = screen.getByRole('region', { name: 'Recently discarded' });
    expect(list).toHaveTextContent(/30 days left/);
    expect(timeLeft(5_000, 0)).toBe('goes within a day');
    expect(timeLeft(2.5 * 24 * 3600 * 1000, 0)).toBe('3 days left');
  });

  it('lists only the first ones and says how many more', () => {
    render(<DiscardedList limit={1} />);
    expect(screen.getAllByRole('listitem')).toHaveLength(1);
    expect(screen.getByText(/And 2 more\./)).toBeInTheDocument();
  });

  it('shows nothing when nothing was discarded', () => {
    useTriageStore.setState({ discarded: [] });
    const { container } = render(<DiscardedList />);
    expect(container).toBeEmptyDOMElement();
  });
});
