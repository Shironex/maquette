import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import type { LayoutCanvasProps } from '../LayoutCanvas';
import LayoutEditor from './LayoutEditor';

const canvas = vi.hoisted(() => ({ props: null as LayoutCanvasProps | null }));
vi.mock('@/features/inspector/LayoutCanvas', () => ({
  default: (props: LayoutCanvasProps) => {
    canvas.props = props;
    return <div data-testid="layout-canvas" />;
  },
}));

beforeEach(async () => {
  await openMockWorkspace();
});
afterEach(() => {
  clearMocks();
  canvas.props = null;
  useLayoutStore.setState({
    view: null,
    project: null,
    selection: null,
    preview: null,
    tool: 'translate',
  });
});

async function editor() {
  render(<LayoutEditor project="moon-island" tabs={<nav>tabs</nav>} />);
  await screen.findByTestId('layout-canvas');
  return canvas.props!;
}

describe('the layout editor', () => {
  it("loads the project's layout into the canvas, loading until it is drawn", async () => {
    const props = await editor();
    expect(screen.getByRole('heading', { name: 'Scene layout' })).toBeInTheDocument();
    expect(screen.getByText('tabs')).toBeInTheDocument();
    expect(props.models).toHaveLength(4);
    expect(props.grid).toBe(true);
    expect(screen.getByText('Loading 4 models…')).toBeInTheDocument();
    act(() => props.handlers.onReady());
    expect(screen.queryByText('Loading 4 models…')).toBeNull();
  });

  it('hides the grid and frames every model again', async () => {
    await editor();
    await userEvent.click(screen.getByRole('button', { name: 'Hide the grid' }));
    expect(canvas.props?.grid).toBe(false);
    await userEvent.click(screen.getByRole('button', { name: 'Frame every model' }));
    expect(canvas.props?.frame).toBe(1);
  });

  it('switches the gizmo with W, E and R, and frames with F', async () => {
    await editor();
    await userEvent.keyboard('e');
    expect(useLayoutStore.getState().tool).toBe('rotate');
    expect(canvas.props?.tool).toBe('rotate');
    await userEvent.keyboard('f');
    expect(canvas.props?.frame).toBe(1);
  });

  it('undoes a drag from the canvas with the undo key', async () => {
    const props = await editor();
    const before = useLayoutStore.getState().draft.stops[0]!.position;
    act(() => {
      props.handlers.onBeginDrag();
      props.handlers.onDragStop('porch', [1, 1, 1]);
    });
    expect(useLayoutStore.getState().draft.stops[0]!.position).toEqual([1, 1, 1]);
    expect(screen.getByText(/unsaved changes/)).toBeInTheDocument();
    await userEvent.keyboard('{Control>}z{/Control}');
    expect(useLayoutStore.getState().draft.stops[0]!.position).toEqual(before);
  });

  it('shows what failed to load over the canvas', async () => {
    const props = await editor();
    act(() => props.handlers.onError('2 of 4 models did not load.'));
    expect(screen.getByText('2 of 4 models did not load.')).toBeInTheDocument();
  });
});
