/** The look the timeline's lanes share: a name column, then the page from top to bottom. */

/** The width of the lane names, in pixels. */
export const NAMES = 180;

export const LANE_NAME =
  'flex h-[34px] items-center gap-1.5 overflow-hidden border-t border-line px-3 text-[12px] whitespace-nowrap text-fg-dim';

export const LANE = 'relative h-[34px] border-t border-l border-line';

export const BLOCK =
  'absolute top-1.5 flex h-[22px] cursor-pointer items-center gap-1.5 overflow-hidden rounded-[5px] border px-2 text-[11.5px] whitespace-nowrap text-fg';

export const KEY =
  'absolute top-3 size-2.5 -ml-[5px] cursor-pointer rounded-[2px] border-0 p-0 rotate-45';

/** Percent of the page as a CSS length along the lane. */
export const pct = (percent: number) => `${percent.toFixed(2)}%`;
