import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';
import KeepConfirm from './KeepConfirm';

const needs = [
  { id: 'c-a1', summary: 'Edited 2 notes: Moon, Porch', parts: ['text'] },
  { id: 'c-a0', summary: 'Edited note: Moon', parts: ['text'] },
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
    expect(dialog).toHaveTextContent('Edited 2 notes: Moon, Porch');
    expect(dialog).toHaveTextContent('Edited note: Moon');
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
    expect(keep).toHaveBeenCalledWith('frostveil', 'combat-ideas', ['c-a2', 'c-a1', 'c-a0']);
  });
});
