import { render, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  FLOATING_BOTTOM,
  floatingBottom,
  useClearanceStore,
  useFloatingLift,
  useKeepClearOfFloating,
} from './useFloatingClearance';

afterEach(() => {
  vi.restoreAllMocks();
  useClearanceStore.setState({ bars: {} });
});

function Bar() {
  const ref = useKeepClearOfFloating<HTMLDivElement>();
  return <div ref={ref}>Add as v2</div>;
}

describe('keeping the floating layers clear of a bottom bar', () => {
  it('sits above the status bar with no bar, and above the highest bar plus a gap', () => {
    expect(floatingBottom({})).toBe(FLOATING_BOTTOM);
    expect(floatingBottom({ a: 120, b: 90 })).toBe(136);
    expect(floatingBottom({ a: 10 })).toBe(FLOATING_BOTTOM);
  });

  it('measures a mounted bar from its top edge and forgets it once unmounted', () => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      top: window.innerHeight - 120,
      height: 56,
    } as DOMRect);
    const { unmount } = render(<Bar />);
    expect(Object.values(useClearanceStore.getState().bars)).toEqual([120]);
    const lift = renderHook(() => useFloatingLift(true)).result.current;
    expect(lift).toEqual({ bottom: 136, maxHeight: 'calc(100vh - 332px)' });
    unmount();
    expect(useClearanceStore.getState().bars).toEqual({});
    expect(renderHook(() => useFloatingLift()).result.current).toBeUndefined();
  });

  it('registers nothing for a bar with no layout', () => {
    render(<Bar />);
    expect(useClearanceStore.getState().bars).toEqual({});
  });
});
