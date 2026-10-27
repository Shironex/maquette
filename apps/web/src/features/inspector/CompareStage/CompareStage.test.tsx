import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { pickSides, type SidePick } from '@/state/inspector';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useMaterialsStore } from '@/state/use-materials-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import type { CompareCanvasProps } from '../CompareCanvas';
import CompareStage, { type CompareStageProps } from './CompareStage';

const canvas = vi.hoisted(() => ({ props: null as CompareCanvasProps | null }));
vi.mock('@/features/inspector/CompareCanvas', () => ({
  default: (props: CompareCanvasProps) => {
    canvas.props = props;
    return <div data-testid="compare-canvas" />;
  },
}));

let pick: SidePick;

beforeEach(async () => {
  await openMockWorkspace();
  pick = pickSides(await commands.inspectProject('frostveil'), null, null, null)!;
});
afterEach(() => {
  clearMocks();
  canvas.props = null;
  useMaterialsStore.setState({ saved: {}, edits: {} });
  useInspectorStore.setState({ fps: 0, mode: 'shaded', stop: null });
});

function stage(over: Partial<CompareStageProps> = {}) {
  const props: CompareStageProps = {
    pick,
    kind: 'scene',
    left: [{ id: 'a', url: 'mock://a', position: [0, 0, 0] }],
    right: [
      { id: 'b', url: 'mock://b', position: [0, 0, 0] },
      { id: 'c', url: 'mock://c', position: [1, 0, 0] },
    ],
    turntable: false,
    stops: [],
    lightmaps: null,
    ready: false,
    loadError: null,
    handlers: {
      onSample: vi.fn(),
      onPick: vi.fn(),
      onStopShots: vi.fn(),
      onUserMove: vi.fn(),
      onLightmaps: vi.fn(),
      onError: vi.fn(),
      onReady: vi.fn(),
    },
    project: 'frostveil',
    ...over,
  };
  return render(<CompareStage {...props} />);
}

describe('the compare stage', () => {
  it('labels both sides and counts the models while they load', async () => {
    stage();
    await screen.findByTestId('compare-canvas');
    expect(screen.getByText('Loading 3 models…')).toBeInTheDocument();
    expect(screen.getByText('v6')).toBeInTheDocument();
    expect(screen.getByText('baked Oct 23')).toBeInTheDocument();
    expect(screen.getByText('v7')).toBeInTheDocument();
    expect(screen.getByText('Free orbit · perspective')).toBeInTheDocument();
    expect(screen.getByText('WebGL 2 · starting')).toBeInTheDocument();
  });

  it('moves the split with its handle and hands it to the canvas', async () => {
    stage({ ready: true });
    await screen.findByTestId('compare-canvas');
    expect(canvas.props?.split).toBe(0.5);
    screen.getByRole('slider', { name: 'Compare position' }).focus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(canvas.props?.split).toBeCloseTo(0.45);
    expect(screen.queryByText(/Loading/)).toBeNull();
  });

  it('shows the frame rate, the stop and a failed load', async () => {
    useInspectorStore.setState({ fps: 59.6, stop: { index: 0, at: 1 } });
    stage({
      loadError: 'The file ends early.',
      stops: [{ name: 'Arrival', position: [0, 1, 4], target: [0, 1, 0], fov: 36, poster: null }],
    });
    await screen.findByTestId('compare-canvas');
    expect(screen.getByText('WebGL 2 · 60 fps')).toBeInTheDocument();
    expect(screen.getByText('Arrival · perspective')).toBeInTheDocument();
    expect(screen.getByText('The file ends early.')).toBeInTheDocument();
    expect(screen.queryByText(/Loading/)).toBeNull();
  });

  it('says so when the lightmap look finds no baked lightmap', async () => {
    stage({ ready: true, lightmaps: 0 });
    await screen.findByTestId('compare-canvas');
    expect(screen.queryByText(/No baked lightmap/)).toBeNull();
    act(() => useInspectorStore.setState({ mode: 'lightmap' }));
    expect(screen.getByText(/No baked lightmap in these models yet/)).toBeInTheDocument();
    expect(canvas.props?.mode).toBe('lightmap');
  });

  it("hands the canvas the project's tweaks, edits over what is saved", async () => {
    const tweak = (tint: string) => ({
      all: { tint, roughness: null, metalness: null, emissiveStrength: null },
      materials: {},
    });
    useMaterialsStore.setState({
      saved: {
        frostveil: { version: 1, models: { 'veil-gate': tweak('#ff0000'), hut: tweak('#00ff00') } },
      },
      edits: { frostveil: { 'veil-gate': tweak('#0000ff') } },
    });
    stage({ ready: true });
    await screen.findByTestId('compare-canvas');
    expect(canvas.props?.looks).toEqual({ 'veil-gate': tweak('#0000ff'), hut: tweak('#00ff00') });
  });
});
