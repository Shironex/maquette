import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { LibraryGripSummary } from '@/ipc/client';
import GripGrid from './GripGrid';

const grip = (id: string, name: string, group: string): LibraryGripSummary => ({
  id,
  uid: `${id}-uid`,
  name,
  version: 'v',
  latest: 2,
  revisions: [],
  weapon: group === 'swords' ? 'longsword' : 'scythe',
  group,
  hands: { holds: 'right', supports: 'left' },
  handsWords: 'both hands',
  pose: { rightHand: [0, 0, 0], leftHand: [0, 0, 0] },
  updatedAt: '2026-11-10T10:00:00Z',
  problem: null,
});

const list = [grip('long', 'Longsword, two hands', 'swords'), grip('scy', 'Scythe', 'polearms')];

describe('GripGrid', () => {
  it('shows each grip with its line and filters by weapon', async () => {
    const onFilter = vi.fn();
    const onSelect = vi.fn();
    const { rerender } = render(
      <GripGrid
        list={list}
        filter="all"
        onFilter={onFilter}
        selected="scy"
        onSelect={onSelect}
        label="Library grips"
      />
    );
    const grid = screen.getByRole('list', { name: 'Library grips' });
    expect(grid).toHaveTextContent('r2 · both hands · 2 bones');
    expect(screen.getByRole('button', { name: 'Scythe' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Longsword, two hands' }));
    expect(onSelect).toHaveBeenCalledWith('long');
    await userEvent.click(screen.getByRole('button', { name: 'Polearms' }));
    expect(onFilter).toHaveBeenCalledWith('polearms');
    rerender(
      <GripGrid
        list={list}
        filter="polearms"
        onFilter={onFilter}
        selected={null}
        onSelect={onSelect}
        label="Library grips"
      />
    );
    expect(screen.queryByRole('button', { name: 'Longsword, two hands' })).toBeNull();
    rerender(
      <GripGrid
        list={list}
        filter="shields"
        onFilter={onFilter}
        selected={null}
        onSelect={onSelect}
        label="Library grips"
      />
    );
    expect(screen.getByText(/No grip for this weapon yet/)).toBeInTheDocument();
  });
});
