import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PlanLogLine } from '@/ipc/client';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import AgentToast from './AgentToast';

const line = (id: string, summary: string): PlanLogLine => ({
  id,
  at: '2026-11-02T12:40:00Z',
  by: 'Research agent',
  run: 'r1',
  summary,
  mark: 'add',
  undone: false,
  undoable: true,
});

afterEach(() => usePlanLogStore.getState().clear());

describe('AgentToast', () => {
  it('says who changed the canvas and undoes the whole run', async () => {
    const lines = [line('c2', 'Moved 2 notes'), line('c1', 'Added note: Dusk light')];
    const undo = vi.fn(async () => true);
    usePlanLogStore.setState({ lines, toast: lines[0], undo });
    render(<AgentToast />);
    expect(screen.getByRole('status')).toHaveTextContent('Research agent changed the canvas');
    expect(screen.getByText('2 changes in this run')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Undo run' }));
    expect(undo).toHaveBeenCalledWith(['c2', 'c1']);
  });

  it('opens the Changes panel or goes when dismissed', async () => {
    const lines = [line('c1', 'Added note: Dusk light')];
    usePlanLogStore.setState({ lines, toast: lines[0] });
    const { rerender } = render(<AgentToast />);
    expect(screen.getByText('Added note: Dusk light')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Show changes' }));
    expect(usePlanLogStore.getState().panel).toBe(true);
    usePlanLogStore.setState({ toast: lines[0] });
    rerender(<AgentToast />);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(usePlanLogStore.getState().dismissed).toBe('c1');
  });
});
