import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { Clip } from '@/ipc/client';
import { useMotionStore } from '@/state/use-motion-store';
import { clearMocks, openMotion } from '@/test/open-motion';
import ClipDetails from './ClipDetails';
import { loopText, rootText } from './ClipDetails.hooks';

afterEach(() => clearMocks());

const clip = (patch: Partial<Clip>): Clip => ({
  name: 'Walk',
  index: 0,
  duration: 1,
  keys: 30,
  frames: 30,
  fps: 30,
  channels: 3,
  targets: 3,
  bytes: 1000,
  shippedBytes: null,
  rootTravel: null,
  loops: true,
  ...patch,
});

describe('the clip details', () => {
  it('reads root motion and looping, or says a packed clip is not readable', () => {
    expect(rootText(clip({ rootTravel: 1.4 }))).toBe('Root motion, 1.40 m');
    expect(rootText(clip({}))).toBe('In place');
    expect(rootText(clip({ loops: null }))).toBe('Not readable, packed');
    expect(loopText(clip({ loops: false }))).toBe('Ends on another pose');
  });

  it('shows the selected clip and whether it ships', async () => {
    await openMotion();
    useMotionStore.getState().setShip('clay-figure', 'Idle breathe', false);
    render(<ClipDetails />);
    expect(screen.getByText('kept off the site')).toBeInTheDocument();
    expect(screen.getByText('Ends where it starts')).toBeInTheDocument();
  });
});
