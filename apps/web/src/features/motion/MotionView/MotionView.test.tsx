import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
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

  it('says nothing moves yet in a still project', async () => {
    await openMotion('still');
    render(<MotionView />);
    expect(await screen.findByRole('region', { name: 'No motion yet' })).toBeInTheDocument();
  });
});
