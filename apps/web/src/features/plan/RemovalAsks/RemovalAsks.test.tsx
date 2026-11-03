import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { RemovalProposal } from '@/ipc/client';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import RemovalAsks from './RemovalAsks';

const ask = (id: string, more: Partial<RemovalProposal> = {}): RemovalProposal => ({
  id,
  at: '2026-11-03T09:00:00Z',
  by: 'Research agent',
  run: 'r-1',
  reason: 'the porch note replaces it.',
  nodes: [{ id: 'n-island', name: 'Old hero idea: the whole island', state: 'here' }],
  stale: false,
  ...more,
});

const show = (proposals: RemovalProposal[]) => {
  const answer = vi.fn(async () => true);
  usePlanExtrasStore.setState({ proposals, busy: false, answer });
  render(<RemovalAsks />);
  return answer;
};

afterEach(() => usePlanExtrasStore.getState().clear());

describe('RemovalAsks', () => {
  it('shows who asks, about which node and why, and answers Keep it or Remove', async () => {
    const answer = show([ask('c-1')]);
    const card = screen.getByRole('group', { name: 'Research agent wants to remove a node' });
    expect(within(card).getByText('Old hero idea: the whole island')).toBeInTheDocument();
    expect(within(card).getByText('It says the porch note replaces it.')).toBeInTheDocument();
    await userEvent.click(within(card).getByRole('button', { name: 'Keep it' }));
    expect(answer).toHaveBeenCalledWith('c-1', false);
    await userEvent.click(
      within(card).getByRole('button', { name: 'Remove: Old hero idea: the whole island' })
    );
    expect(answer).toHaveBeenLastCalledWith('c-1', true);
  });

  it('only keeps a stale ask, and says which node changed', () => {
    show([
      ask('c-2', {
        reason: '',
        stale: true,
        nodes: [
          { id: 'n-a', name: 'Porch', state: 'changed' },
          { id: 'n-b', name: 'Moon', state: 'gone' },
        ],
      }),
    ]);
    const card = screen.getByRole('group', { name: 'Research agent wants to remove 2 nodes' });
    expect(within(card).getByText('(changed since)')).toBeInTheDocument();
    expect(within(card).getByText('(gone)')).toBeInTheDocument();
    expect(within(card).getByText(/changed or went since it asked/)).toBeInTheDocument();
    expect(within(card).getByRole('button', { name: /^Remove/ })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(within(card).queryByText(/It says/)).not.toBeInTheDocument();
  });

  it('shows nothing when no agent is waiting', () => {
    show([]);
    expect(screen.queryByRole('region', { name: 'Waiting for you' })).not.toBeInTheDocument();
  });
});
