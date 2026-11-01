import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import MotionBarActions from './MotionBarActions';

afterEach(() => clearMocks());

describe('the motion bar actions', () => {
  it('shows no zero while the motion is being read', async () => {
    await openMotion();
    useMotionStore.setState({ view: null, error: null });
    render(<MotionBarActions />);
    expect(screen.getByRole('meter')).toHaveAccessibleName('Scene budget, Reading...');
    expect(screen.queryByText('0.00')).toBeNull();
  });

  it('does not say it is reading with no project open', async () => {
    await openMotion();
    const workspace = useAppStore.getState().workspace!;
    useAppStore.setState({ workspace: { ...workspace, project: null } });
    useMotionStore.setState({ view: null, error: null });
    render(<MotionBarActions />);
    expect(screen.getByRole('meter')).toHaveAccessibleName('Scene budget, No project');
  });

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
