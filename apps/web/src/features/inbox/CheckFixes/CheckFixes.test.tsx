import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import type { Check, InboxItem } from '@/ipc/client';
import { usePrepareStore } from '@/state/use-prepare-store';
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
});
