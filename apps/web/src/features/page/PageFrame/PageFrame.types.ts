import type { ReactNode } from 'react';
import type { DeviceId } from '@/state/page';

/** What the address bar's badge says about the page. */
export type FrameBadge = 'live' | 'poster' | 'concept' | 'run';

export interface PageFrameProps {
  /** Names the frame in the store, for what its page reports. */
  id: string;
  /** What the frame loads; null shows `children` instead. */
  src: string | null;
  /** The address bar's text. */
  address: string;
  badge: FrameBadge;
  device: DeviceId;
  /**
   * `main` (the default) fits the whole viewport with the fit label; `tile`
   * does the same in a tighter bar without it; `column` fills the frame's
   * width and crops below, as the compare board's columns do.
   */
  layout?: 'main' | 'tile' | 'column';
  /** Names the page for screen readers: "the home page from the site dev page". */
  label: string;
  /** Shows the current stop on the page. */
  showStop?: boolean;
  /** A still image instead of a page, for this run's poster. */
  image?: string | null;
  /** Controls at the end of the address bar, before Reload: the main frame's Stop. */
  actions?: ReactNode;
  /** Shown in the viewport while there is no page: the server's state. */
  children?: ReactNode;
  className?: string;
}
