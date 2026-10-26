import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import MotionBarActions from './MotionBarActions';

afterEach(() => clearMocks());

describe('the motion bar actions', () => {
  it('saves and reverts only with changes', async () => {
    await openMotion();
    render(<MotionBarActions />);
    expect(screen.getByRole('button', { name: 'Save motion' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    useMotionStore.getState().setShip('clay-figure', 'Wave', false);
    fireEvent.click(await screen.findByRole('button', { name: 'Revert' }));
    expect(useMotionStore.getState().draft.clipsOff).toEqual({});
  });
});
