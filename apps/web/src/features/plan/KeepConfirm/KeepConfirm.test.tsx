import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';
import KeepConfirm from './KeepConfirm';

const needs = [
  { id: 'c-a1', at: '2026-10-31T09:12:00.000Z', summary: 'Edited note: Moon', parts: ['text'] },
  { id: 'c-a0', at: '2026-10-31T09:10:00.000Z', summary: 'Edited note: Moon', parts: ['text'] },
];

const open = () =>
  usePlansStore.setState({
    project: 'frostveil',
    open: { id: 'combat-ideas' } as ReturnType<typeof usePlansStore.getState>['open'],
  });

afterEach(() => {
  vi.restoreAllMocks();
  usePlanLogStore.getState().clear();
});

describe('KeepConfirm', () => {
  it('asks before keeping the earlier changes a keep sits on, and Cancel keeps nothing', async () => {
    open();
    vi.spyOn(commands, 'planKeepNeeds').mockResolvedValue(needs);
    const keep = vi.spyOn(commands, 'planKeep').mockResolvedValue(null);
    render(<KeepConfirm />);
    expect(await usePlanLogStore.getState().keep(['c-a2'])).toBe(false);
    const dialog = await screen.findByRole('dialog', { name: 'Keep the earlier changes too?' });
    expect(dialog).toHaveTextContent('Keeping this also keeps');
    // Two changes with the same summary tell apart by their time and id.
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent(/Edited note: Moon \(text, .+, c-a1\)/);
    expect(items[1]).toHaveTextContent(/c-a0/);
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(keep).not.toHaveBeenCalled();
  });

  it('keeps every listed change with Keep all', async () => {
    open();
    vi.spyOn(commands, 'planKeepNeeds').mockResolvedValue(needs);
    const keep = vi.spyOn(commands, 'planKeep').mockResolvedValue(null);
    vi.spyOn(commands, 'planLog').mockResolvedValue([]);
    render(<KeepConfirm />);
    await usePlanLogStore.getState().keep(['c-a2']);
    await userEvent.click(await screen.findByRole('button', { name: 'Keep all' }));
    expect(keep).toHaveBeenCalledWith('frostveil', 'combat-ideas', ['c-a2'], ['c-a1', 'c-a0']);
  });

  it('says the list changed when it moved under the dialog, and keeps nothing', async () => {
    open();
    vi.spyOn(commands, 'planKeepNeeds').mockResolvedValue(needs);
    vi.spyOn(commands, 'planKeep').mockRejectedValue({
      code: 'invalid',
      message: 'The list changed since you saw it, so nothing was kept. Review again.',
    });
    const log = vi.spyOn(commands, 'planLog').mockResolvedValue([]);
    render(<KeepConfirm />);
    await usePlanLogStore.getState().keep(['c-a2']);
    await userEvent.click(await screen.findByRole('button', { name: 'Keep all' }));
    await waitFor(() => expect(log).toHaveBeenCalled());
    expect(useAppStore.getState().notice?.detail).toMatch(/The list changed/);
  });
});
