import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Check, InboxItem } from '@/ipc/client';
import { usePrepareStore } from '@/state/use-prepare-store';
import { useRigStore } from '@/state/use-rig-store';
import { useSplitStore } from '@/state/use-split-store';
import { CheckFixes } from '.';

const item = { id: 'lamp.glb' } as InboxItem;
const check: Check = {
  id: 'triangles',
  status: 'warning',
  title: '94.6k triangles',
  detail: null,
  fixes: [{ op: 'decimate', triangles: 60_000 }],
};

describe('CheckFixes', () => {
  beforeEach(() => usePrepareStore.setState({ drafts: {} }));

  it('adds the fix to the draft and takes it out again', async () => {
    render(<CheckFixes item={item} check={check} />);
    await userEvent.click(screen.getByRole('button', { name: 'Decimate to 60k' }));
    expect(usePrepareStore.getState().drafts['lamp.glb']?.fixups).toEqual([
      { op: 'decimate', triangles: 60_000 },
    ]);
    const added = screen.getByRole('button', { name: 'Decimate to 60k added' });
    expect(added).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(added);
    expect(usePrepareStore.getState().drafts['lamp.glb']).toBeUndefined();
  });

  it('offers a split for a prop that is one mesh', async () => {
    const merged: Check = {
      id: 'merged',
      status: 'warning',
      title: 'One mesh, no parts',
      detail: null,
      fixes: [
        { op: 'split-parts', by: 'loose', mergeUnder: 0.01 },
        { op: 'remove-loose', minShare: 0.02 },
      ],
    };
    render(<CheckFixes item={item} check={merged} />);
    await userEvent.click(screen.getByRole('button', { name: 'Split into parts' }));
    expect(screen.getByRole('button', { name: 'Split into parts added' })).toBeInTheDocument();
    expect(usePrepareStore.getState().drafts['lamp.glb']?.fixups).toEqual([
      { op: 'split-parts', by: 'loose', mergeUnder: 0.01 },
    ]);
  });

  it('opens the provider split sheet for one mesh from a send', async () => {
    const merged: Check = {
      id: 'merged',
      status: 'warning',
      title: 'One mesh, no parts',
      detail: null,
      fixes: [],
    };
    const sent = {
      id: 'frost-lantern-2.glb',
      link: {
        job: 'frost-lantern-2',
        project: 'frostveil',
        asset: 'frost-lantern',
        assetName: 'Frost lantern',
        provider: 'tripo',
      },
    } as InboxItem;
    render(<CheckFixes item={item} check={merged} />);
    expect(screen.queryByRole('button', { name: 'Split on Tripo or Meshy...' })).toBeNull();
    render(<CheckFixes item={sent} check={merged} />);
    await userEvent.click(screen.getByRole('button', { name: 'Split on Tripo or Meshy...' }));
    expect(useSplitStore.getState().split).toEqual({
      project: 'frostveil',
      asset: 'frost-lantern',
      version: null,
      item: 'frost-lantern-2.glb',
    });
  });

  it('opens the rig doctor from a rig check that did not pass, and not from one that did', async () => {
    const show = vi.fn().mockResolvedValue(undefined);
    useRigStore.setState({ show });
    const rig: Check = { id: 'finger-weights', status: 'fail', title: '', detail: null, fixes: [] };
    const { rerender } = render(<CheckFixes item={item} check={rig} />);
    await userEvent.click(screen.getByRole('button', { name: 'Fix in rig doctor' }));
    expect(show).toHaveBeenCalledWith(
      {
        inbox: 'lamp.glb',
        project: null,
        asset: null,
        version: null,
      },
      item.name
    );
    rerender(<CheckFixes item={item} check={{ ...rig, status: 'pass' }} />);
    expect(screen.queryByRole('button', { name: 'Fix in rig doctor' })).toBeNull();
  });
});
