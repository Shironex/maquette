import { useCallback, useLayoutEffect, useReducer, useRef, useState, type RefObject } from 'react';
import {
  Virtualizer,
  elementScroll,
  measureElement,
  observeElementOffset,
  observeElementRect,
} from '@tanstack/react-virtual';

/** Lists up to this many items render whole: windowing only pays off past it. */
export const WINDOW_FROM = 60;

/**
 * The size a scroller or row reports before layout, as in a test DOM; a
 * window of rows still renders against it.
 */
const FALLBACK = { width: 1024, height: 800 };

export interface WindowedListOptions {
  /** How many items the list holds. */
  count: number;
  /** The element that scrolls the list. */
  scrollRef: RefObject<HTMLElement | null>;
  /** The list's own element, when something sits above it in the same scroller. */
  listRef?: RefObject<HTMLElement | null>;
  /** A row's height in pixels until it is measured. */
  estimate: number;
  /** Pixels between rows. */
  gap?: number;
  /** Items per row, for a grid. */
  columns?: number;
  /** Lists up to this many items render whole. */
  threshold?: number;
  /** Rows rendered past each edge of the view. */
  overscan?: number;
}

/** What an item spreads on its outer element; only a row's first item measures the row. */
export interface WindowItemProps {
  ref?: (el: Element | null) => void;
  'data-index'?: number;
}

export interface WindowedList {
  /** False while the list renders whole. */
  windowed: boolean;
  /** The first item to render. */
  start: number;
  /** One past the last item to render. */
  end: number;
  /** Pixels of the rows not rendered above the window and below it. */
  before: number;
  after: number;
  /**
   * Props for item `index`: the first item of each row carries the ref that
   * measures the row. Spread them on the item's outer element.
   */
  itemProps: (index: number) => WindowItemProps;
  /** Scrolls the row holding item `index` into view, by the least, or to its end. */
  scrollTo: (index: number, align?: 'auto' | 'end') => void;
}

interface Slice {
  start: number;
  end: number;
  before: number;
  after: number;
}

const WHOLE: Slice = { start: 0, end: 0, before: 0, after: 0 };

const sameSlice = (a: Slice, b: Slice) =>
  a.start === b.start && a.end === b.end && a.before === b.before && a.after === b.after;

/** The rows the virtualizer has in view, as item indexes and the space around them. */
function sliceOf(v: Virtualizer<HTMLElement, Element>, columns: number, count: number): Slice {
  const rows = v.getVirtualItems();
  const first = rows[0];
  const last = rows[rows.length - 1];
  if (!first || !last) return WHOLE;
  const margin = v.options.scrollMargin;
  return {
    start: first.index * columns,
    end: Math.min(count, (last.index + 1) * columns),
    before: Math.max(0, first.start - margin),
    after: Math.max(0, v.getTotalSize() - (last.end - margin)),
  };
}

/**
 * The element `ref` points at, kept in state so effects run again when it
 * appears late (a list rendered once its data loads) or is swapped for
 * another (a grid switched to a list).
 */
function useElement<T extends HTMLElement>(ref: RefObject<T | null>): T | null {
  // A reducer, not state: React drops a dispatch of the same element, so this cannot loop.
  const [el, see] = useReducer((_: T | null, next: T | null) => next, null);
  useLayoutEffect(() => see(ref.current));
  return el;
}

/** Where the list starts inside its scroller's content. */
function marginOf(scroller: HTMLElement, list: HTMLElement): number {
  return (
    list.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop
  );
}

/**
 * Renders only the rows of a long list that are in view, plus a few each
 * side, with the space of the others kept as padding (or spacer rows in a
 * table) so the scrollbar stays true. Rows are measured as they render, so
 * rows that wrap or grow keep their place. The component re-renders only
 * when the window moves, not on every scroll event.
 */
export function useWindowedList(options: WindowedListOptions): WindowedList {
  const { count, scrollRef, listRef, estimate, gap = 0, overscan = 4 } = options;
  const columns = Math.max(1, options.columns ?? 1);
  const windowed = count > (options.threshold ?? WINDOW_FROM);
  const rows = windowed ? Math.ceil(count / columns) : 0;
  const [, moved] = useReducer((n: number) => n + 1, 0);
  const shown = useRef<Slice>(WHOLE);
  const [margin, setMargin] = useState(0);
  const [v] = useState(
    () =>
      new Virtualizer<HTMLElement, Element>({
        count: 0,
        getScrollElement: () => null,
        estimateSize: () => estimate,
        scrollToFn: elementScroll,
        observeElementRect,
        observeElementOffset,
        initialRect: FALLBACK,
      })
  );
  v.setOptions({
    ...v.options,
    count: rows,
    enabled: windowed,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => estimate,
    gap,
    overscan,
    scrollMargin: margin,
    observeElementRect: (instance, cb) =>
      observeElementRect(instance, rect => cb(rect.height > 0 ? rect : FALLBACK)),
    measureElement: (el, entry, instance) => measureElement(el, entry, instance) || estimate,
    // Scroll events land here: render again only when the window moved.
    onChange: instance => {
      if (!sameSlice(shown.current, sliceOf(instance, columns, count))) moved();
    },
  });

  const scrollTo = useCallback(
    (index: number, align: 'auto' | 'end' = 'auto') => {
      if (v.options.count > 0) v.scrollToIndex(Math.floor(index / columns), { align });
    },
    [v, columns]
  );

  useLayoutEffect(() => v._didMount(), [v]);
  useLayoutEffect(() => {
    shown.current = sliceOf(v, columns, count);
  });
  useLayoutEffect(() => v._willUpdate());
  // What sits above the list can change height; the list's own changes are cheap to re-read.
  useLayoutEffect(() => {
    const scroller = scrollRef.current;
    const list = listRef?.current;
    if (!windowed || !scroller || !list) return;
    const read = () => setMargin(marginOf(scroller, list));
    read();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(read);
    observer.observe(scroller);
    observer.observe(list);
    return () => observer.disconnect();
  }, [windowed, scrollRef, listRef]);

  if (!windowed) {
    return {
      windowed,
      start: 0,
      end: count,
      before: 0,
      after: 0,
      itemProps: () => ({}),
      scrollTo,
    };
  }
  const slice = sliceOf(v, columns, count);
  return {
    windowed,
    ...slice,
    itemProps: index =>
      index % columns === 0 ? { ref: v.measureElement, 'data-index': index / columns } : {},
    scrollTo,
  };
}

/** How many columns of at least `min` pixels, `gap` apart, fit across `ref`'s width. */
export function useGridColumns(
  ref: RefObject<HTMLElement | null>,
  min: number,
  gap: number
): number {
  const [columns, setColumns] = useState(1);
  const el = useElement(ref);
  useLayoutEffect(() => {
    if (!el) return;
    const read = () => {
      const width = el.clientWidth || FALLBACK.width;
      setColumns(Math.max(1, Math.floor((width + gap) / (min + gap))));
    };
    read();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, [el, min, gap]);
  return columns;
}
