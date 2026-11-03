import { describe, expect, it, vi } from 'vitest';
import {
  FOLDED_INSET,
  MAP_FOLD_WIDTH,
  READABLE_ZOOM,
  SAFE_INSET,
  fitPadding,
  mapShown,
  reveal,
} from './plan-view';

describe('the minimap on a narrow canvas', () => {
  it('folds below the fold width unless the person chose, and counts an unmeasured canvas as wide', () => {
    expect(mapShown(MAP_FOLD_WIDTH, null)).toBe(true);
    expect(mapShown(MAP_FOLD_WIDTH - 1, null)).toBe(false);
    expect(mapShown(0, null)).toBe(true);
    expect(mapShown(600, true)).toBe(true);
    expect(mapShown(1400, false)).toBe(false);
  });

  it('gives back the bottom room the map took to a fit', () => {
    expect(fitPadding(true).bottom).toBe(`${SAFE_INSET.bottom + 8}px`);
    expect(fitPadding(false).bottom).toBe(`${FOLDED_INSET.bottom + 8}px`);
    expect(fitPadding(false).left).toBe(fitPadding(true).left);
  });
});

describe('revealing the changed nodes', () => {
  // A 1000 by 800 canvas: the safe part is 908 by 586 pixels.
  const frame = {
    current: {
      getBoundingClientRect: () => ({ left: 0, top: 0, right: 1000, bottom: 800, width: 1000 }),
    },
  } as unknown as Parameters<typeof reveal>[1];
  const fakeFlow = (
    boxes: Record<string, { x: number; y: number; width: number; height: number }>
  ) => {
    const setCenter = vi.fn();
    const fitView = vi.fn();
    const flow = {
      getNode: (id: string) => boxes[id],
      getNodesBounds: (ids: string[]) => {
        const all = ids.flatMap(id => boxes[id] ?? []);
        const x = Math.min(...all.map(b => b.x));
        const y = Math.min(...all.map(b => b.y));
        return {
          x,
          y,
          width: Math.max(...all.map(b => b.x + b.width)) - x,
          height: Math.max(...all.map(b => b.y + b.height)) - y,
        };
      },
      // Zoom 1, the view at the flow's origin.
      flowToScreenPosition: (p: { x: number; y: number }) => p,
      getZoom: () => 1,
      setCenter,
      fitView,
    } as unknown as Parameters<typeof reveal>[0];
    return { flow, setCenter, fitView };
  };
  const node = (x: number, y: number) => ({ x, y, width: 240, height: 120 });

  it('zooms out as far as the controls stay readable to show them all', () => {
    const { flow, setCenter, fitView } = fakeFlow({ a: node(2000, 100), b: node(2700, 100) });
    reveal(flow, frame, ['a', 'b'], true);
    expect(fitView).not.toHaveBeenCalled();
    const [, , options] = setCenter.mock.calls[0] as [number, number, { zoom: number }];
    expect(options.zoom).toBeCloseTo((1000 - SAFE_INSET.left - SAFE_INSET.right) / 940);
    expect(options.zoom).toBeGreaterThanOrEqual(READABLE_ZOOM);
  });

  it('brings the first one into view when they are too far apart to read', () => {
    const { flow, setCenter } = fakeFlow({ a: node(2000, 100), b: node(6000, 100) });
    reveal(flow, frame, ['a', 'b'], true);
    const [x, , options] = setCenter.mock.calls[0] as [number, number, { zoom: number }];
    expect(options.zoom).toBe(1);
    const shift = (SAFE_INSET.left - SAFE_INSET.right) / 2;
    expect(x).toBe(2000 + 120 - shift);
  });

  it('counts the bottom the folded map gave back as in sight', () => {
    // At the bottom edge of the shown map's safe part, inside the folded one's.
    const { flow, setCenter } = fakeFlow({ a: node(200, 600) });
    reveal(flow, frame, ['a'], true, false);
    expect(setCenter).not.toHaveBeenCalled();
    reveal(flow, frame, ['a'], true, true);
    expect(setCenter).toHaveBeenCalled();
  });

  it('leaves the view alone when they are in sight', () => {
    const { flow, setCenter } = fakeFlow({ a: node(200, 200) });
    reveal(flow, frame, ['a'], true);
    expect(setCenter).not.toHaveBeenCalled();
  });
});
