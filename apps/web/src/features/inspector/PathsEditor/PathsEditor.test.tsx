import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePathsStore } from '@/state/use-paths-store';
import { clearMocks, openPaths } from '@/test/open-paths';
import type { PathsCanvasProps } from '../PathsCanvas';
import PathsEditor from './PathsEditor';

const canvas = vi.hoisted(() => ({ props: null as PathsCanvasProps | null }));
vi.mock('@/features/inspector/PathsCanvas', () => ({
  default: (props: PathsCanvasProps) => {
    canvas.props = props;
    return <div data-testid="paths-canvas" />;
  },
}));

afterEach(() => {
  clearMocks();
  canvas.props = null;
});

async function editor(paths = 'demo') {
  await openPaths(paths);
  render(<PathsEditor project="frostveil" tabs={<nav>tabs</nav>} />);
  await screen.findByTestId('paths-canvas');
  return canvas.props!;
}

describe('the paths editor', () => {
  it('draws every path and the first one picked, with its points', async () => {
    const props = await editor();
    expect(screen.getByRole('heading', { name: 'Scene paths' })).toBeInTheDocument();
    expect(props.paths.map(p => p.id)).toEqual(['rune-line', 'root-glow', 'spare-run']);
    expect(props.selection).toEqual({ path: 'rune-line', point: null });
    expect(screen.getByText(/Rune line · 5 points/)).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /^Pick point/ })).toHaveLength(5);
    expect(screen.getByText('Follows Observatory arch')).toBeInTheDocument();
  });

  it('adds a point after the picked one where the canvas was clicked', async () => {
    const props = await editor();
    await userEvent.click(screen.getByRole('button', { name: /^Add points/ }));
    act(() => canvas.props!.handlers.onSelect({ path: 'rune-line', point: 1 }));
    act(() => props.handlers.onAddPoint([1, 0, 4]));
    const cable = usePathsStore.getState().draft.paths[0]!;
    expect(cable.points[2]).toEqual([1, 0, 4]);
    expect(usePathsStore.getState().selection).toEqual({ path: 'rune-line', point: 2 });
  });

  it('moves a dragged point at once and saves it when let go', async () => {
    const props = await editor();
    act(() => props.handlers.onDragPoint(1, [-2, 0.5, 4.8]));
    expect(usePathsStore.getState().draft.paths[0]!.points[1]).toEqual([-2, 0.5, 4.8]);
    act(() => props.handlers.onDropPoint(1, [-2, 0.6, 4.8]));
    await act(() => usePathsStore.getState().save());
    expect(usePathsStore.getState().saved.paths[0]!.points[1]).toEqual([-2, 0.6, 4.8]);
  });

  it('removes the picked point with Delete, keeping two', async () => {
    await editor();
    act(() => usePathsStore.getState().select({ path: 'spare-run', point: 0 }));
    await userEvent.keyboard('{Delete}');
    expect(usePathsStore.getState().draft.paths[2]!.points).toHaveLength(2);
    act(() => usePathsStore.getState().select({ path: 'rune-line', point: 4 }));
    await userEvent.keyboard('{Delete}');
    const cable = usePathsStore.getState().draft.paths[0]!;
    expect(cable.points).toHaveLength(4);
    expect(cable.end).toBeNull();
  });

  it('offers a first path when there is none', async () => {
    await editor('empty');
    expect(screen.getByText('No paths yet', { selector: 'h2, h3, p, span, div' })).toBeTruthy();
    await userEvent.click(screen.getAllByRole('button', { name: 'New path' })[0]!);
    const { draft, selection } = usePathsStore.getState();
    expect(draft.paths.map(p => p.name)).toEqual(['Path 1']);
    expect(selection).toEqual({ path: 'path-1', point: null });
  });

  it('lays a stroke from Draw on surfaces after the picked point', async () => {
    const props = await editor();
    await userEvent.click(screen.getByRole('button', { name: /^Draw on surfaces/ }));
    expect(usePathsStore.getState().tool).toBe('surface');
    expect(canvas.props!.tool).toBe('surface');
    act(() => canvas.props!.handlers.onSelect({ path: 'rune-line', point: 1 }));
    act(() =>
      props.handlers.onStroke([
        [1, 1, 1],
        [2, 1, 1],
        [3, 1, 2],
      ])
    );
    const s = usePathsStore.getState();
    expect(s.draft.paths[0]!.points[2]).toEqual([1, 1, 1]);
    expect(s.draft.paths[0]!.points.length).toBe(8);
    expect(s.selection).toEqual({ path: 'rune-line', point: 4 });
  });
});
