import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openGameMotion, openMotion } from '@/test/open-motion';
import MotionView from './MotionView';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => clearMocks());

describe('the motion view', () => {
  it('shows the list, the clip stage and the clips panel', async () => {
    await openMotion();
    render(<MotionView />);
    expect(await screen.findByRole('region', { name: 'Models that move' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Preview' })).toBeInTheDocument();
    expect(screen.getByRole('complementary', { name: 'Clips and rig' })).toBeInTheDocument();
  });

  it('shows the states board for a prop with states', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<MotionView />);
    expect(await screen.findByRole('complementary', { name: 'State details' })).toBeInTheDocument();
    expect(useMotionStore.getState().tab).toBe('states');
  });

  it('shows a game the model budget and the game stages, with clips only', async () => {
    await openGameMotion();
    render(<MotionView />);
    expect(
      await screen.findByRole('meter', { name: /^Game budget, \d+ of \d+ models over$/ })
    ).toBeVisible();
    for (const label of ['Concept', 'Refs', 'Tripo', 'Blender', 'Export', 'Ship']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.queryByText('Web')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
    expect(screen.queryByRole('tab', { name: 'States' })).toBeNull();
  });

  it('says nothing moves yet in a still project', async () => {
    await openMotion('still');
    render(<MotionView />);
    expect(await screen.findByRole('region', { name: 'No motion yet' })).toBeInTheDocument();
  });
});
