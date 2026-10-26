import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import BoneMapDialog from './BoneMapDialog';
import { mapCounts } from './BoneMapDialog.hooks';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => clearMocks());

describe('the bone map', () => {
  it('counts matched and missing bones', () => {
    expect(
      mapCounts([
        { bone: 'Hips', joint: 'hips', how: 'exact' },
        { bone: 'Spine', joint: 'spine_01', how: 'by-name' },
        { bone: 'LeftToeBase', joint: null, how: 'missing' },
      ])
    ).toEqual({ matched: 2, missing: 1 });
  });

  it('shows the rig map, filters to the missing bones, and cannot retarget without a library', async () => {
    await openMotion();
    useMotionStore.getState().setBoneMap(true);
    render(<BoneMapDialog />);
    expect(screen.getByRole('dialog', { name: 'Retarget a library clip onto Clay figure' }));
    expect(screen.getByText('No motion library yet')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Missing' }));
    expect(screen.getAllByText('none, stays at rest')).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Retarget' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(useMotionStore.getState().boneMap).toBe(false);
  });
});
