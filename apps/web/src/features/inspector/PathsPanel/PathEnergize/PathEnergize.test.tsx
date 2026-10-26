import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { LayoutStop, ScenePath } from '@/ipc/client';
import PathEnergize from './PathEnergize';

const PATH: ScenePath = {
  id: 'cord',
  name: 'Cord',
  curve: 'polyline',
  points: [
    [0, 0, 0],
    [1, 0, 0],
  ],
  start: null,
  end: null,
  width: 0.02,
  color: '#3a3936',
  glow: '#ffe08a',
};
const STOP: LayoutStop = {
  id: 'porch',
  name: 'Porch',
  position: [0, 1, 2],
  target: [0, 0, 0],
  fov: 35,
  models: null,
};

describe('the energize section', () => {
  it('needs a camera stop before a path can light', () => {
    render(<PathEnergize path={PATH} track={undefined} stops={[]} change={vi.fn()} />);
    expect(screen.getByRole('switch', { name: /Light it/ })).toBeDisabled();
    expect(screen.getByText(/Add a camera stop in the Layout tab first/)).toBeInTheDocument();
  });

  it('says where the fill is keyed and keeps the unbuilt drivers off', () => {
    const track = { easing: 'linear' as const, keys: [{ stop: 'porch', amount: 0.5 }] };
    render(<PathEnergize path={PATH} track={track} stops={[STOP]} change={vi.fn()} />);
    expect(screen.getByText('50% at Porch')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Scroll range/ })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByText(/Speed and direction are not built yet/)).toBeInTheDocument();
  });
});
