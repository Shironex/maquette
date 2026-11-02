import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { CameraView, PlanNode } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import StopBody from './StopBody';

const node = (id: string, stopId: string, name: string, by: string | null = null): PlanNode => ({
  id,
  kind: 'stop',
  x: 0,
  y: 0,
  width: null,
  title: '',
  text: '',
  image: null,
  url: null,
  asset: null,
  direction: null,
  by,
  stop: { id: stopId, name, position: [1, 2, 3], target: [0, 0, 0], fov: 32, models: null },
});

const nodes = [
  node('a', 'porch', 'Porch'),
  node('b', 'moon', 'Moon'),
  node('p', 'moon', 'Moon', 'Camera agent'),
  node('q', 'lantern', 'Lantern', 'Camera agent'),
];

afterEach(() => usePlanCameraStore.getState().clear());

describe('StopBody', () => {
  it('shows a main line stop by its place, and a proposal stop by where it was', () => {
    registerMockFile('/p/web/posters/porch.webp', 'data:image/webp;base64,AAAA');
    usePlanCameraStore.setState({
      view: {
        main: ['a', 'b'],
        lines: [{ nodes: ['p', 'q'], agent: 'Camera agent', problem: null }],
        posters: [{ stop: 'porch', poster: '/p/web/posters/porch.webp' }],
        doc: { plan: { nodes } },
      } as unknown as CameraView,
    });
    const { unmount } = render(<StopBody node={nodes[0]!} />);
    expect(screen.getByText('Stop 1')).toBeInTheDocument();
    expect(screen.getByText('first view')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Render of Porch' })).toBeInTheDocument();
    expect(screen.getByText('32°')).toBeInTheDocument();
    unmount();
    render(<StopBody node={nodes[2]!} />);
    expect(screen.getByText('Proposal 1')).toBeInTheDocument();
    expect(screen.getByText('was stop 2')).toBeInTheDocument();
    expect(screen.getByText('No render yet')).toBeInTheDocument();
  });
});
