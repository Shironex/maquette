import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockGame } from '@/test/mock-ipc';
import { BudgetPanel } from '.';

beforeEach(async () => {
  await openMockGame('crypt');
});
afterEach(() => clearMocks());

describe('BudgetPanel', () => {
  it('flags the models over their folder budget', async () => {
    render(<BudgetPanel />);
    const over = screen.getByRole('button', { name: /Over budget/ });
    await userEvent.click(over);
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows.map(r => within(r).getAllByRole('cell')[0]?.textContent)).toEqual([
      'Bannerprops/banner.glb',
      'Iron gateprops/iron_gate.glb',
      'Knightenemies/knight.glb',
    ]);
    expect(screen.getByRole('button', { name: 'Add the 2 fix-ups' })).toBeInTheDocument();
  });

  it('never offers Decimate on a rigged model, only Shrink textures', async () => {
    render(<BudgetPanel />);
    await userEvent.click(screen.getByRole('button', { name: /Over budget/ }));
    const knight = screen.getAllByRole('row').find(r => r.textContent?.startsWith('Knight'))!;
    const fix = within(knight).getAllByRole('cell').at(-1)!;
    expect(fix).toHaveTextContent('By hand');
    expect(fix).toHaveAttribute('title', expect.stringMatching(/^Rigged: reduce by hand/));
    expect(within(knight).queryByText('Decimate')).toBeNull();
    expect(screen.getByText(/Rigged: reduce by hand\. Decimate would tear/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add the 2 fix-ups' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice?.title).toBe('Fix-ups added to 2 models')
    );
  });

  it('saves a folder budget', async () => {
    render(<BudgetPanel />);
    const field = screen.getByRole('textbox', { name: 'enemies/ triangles' });
    await userEvent.clear(field);
    await userEvent.type(field, '50000');
    await userEvent.tab();
    await userEvent.click(screen.getByRole('button', { name: 'Save budgets' }));
    const budgets = useAppStore.getState().workspace?.project?.game?.budgets;
    expect(budgets?.find(b => b.folder === 'enemies')?.triangles).toBe(50_000);
  });
});
