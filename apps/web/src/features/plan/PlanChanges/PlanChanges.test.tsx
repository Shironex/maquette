import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PlanLogLine } from '@/ipc/client';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import PlanChanges from './PlanChanges';

const line = (id: string, by: string | null, summary: string, more = {}): PlanLogLine => ({
  id,
  at: '2026-11-02T12:40:00Z',
  by,
  run: by ? 'r1' : null,
  summary,
  mark: 'add',
  undone: false,
  undoable: true,
  ...more,
});

const show = (lines: PlanLogLine[]) => {
  const undo = vi.fn(async () => true);
  usePlanLogStore.setState({ lines, error: null, filter: 'all', panel: true, undo });
  render(<PlanChanges />);
  return undo;
};

afterEach(() => usePlanLogStore.getState().clear());

describe('PlanChanges', () => {
  it('lists an agent run open with its undo, and a person run folded', async () => {
    const undo = show([
      line('c3', 'Research agent', 'Added note: Dusk light'),
      line('c2', 'Research agent', 'Moved 3 notes', { mark: 'move' }),
      line('c1', null, 'Edited note: Frost veil', { mark: 'edit' }),
    ]);
    const run = screen.getByRole('group', { name: 'Research agent, 2 changes' });
    expect(within(run).getByText('Added note: Dusk light')).toBeInTheDocument();
    const mine = screen.getByRole('group', { name: 'You, 1 change' });
    expect(within(mine).queryByText('Edited note: Frost veil')).not.toBeInTheDocument();
    await userEvent.click(within(mine).getByRole('button', { name: 'Show the changes' }));
    expect(within(mine).getByText('Edited note: Frost veil')).toBeInTheDocument();

    await userEvent.click(within(run).getByRole('button', { name: 'Undo run' }));
    expect(undo).toHaveBeenCalledWith(['c3', 'c2']);
    await userEvent.click(within(run).getByRole('button', { name: 'Undo: Moved 3 notes' }));
    expect(undo).toHaveBeenLastCalledWith(['c2']);
  });

  it('marks undone changes and filters to agents or mine', async () => {
    show([
      line('c2', 'Research agent', 'Added note: Dusk light', { undone: true, undoable: false }),
      line('c1', null, 'Made the canvas Combat ideas', { mark: 'create', undoable: false }),
    ]);
    expect(screen.getByText('undone')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Undo/ })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Mine' }));
    expect(screen.queryByRole('group', { name: /^Research agent/ })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Agents' }));
    expect(screen.queryByRole('group', { name: /^You/ })).not.toBeInTheDocument();
  });

  it('says when the log is empty or unreadable, and closes', async () => {
    show([]);
    expect(screen.getByText(/No changes yet/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Close Changes' }));
    expect(usePlanLogStore.getState().panel).toBe(false);
    usePlanLogStore.setState({ error: 'Locked' });
    expect(await screen.findByRole('alert')).toHaveTextContent('Locked');
  });
});
