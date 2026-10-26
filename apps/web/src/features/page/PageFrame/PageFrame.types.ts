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
  /** `contain` fits the whole viewport; `width` fills the frame's width and crops below. */
  fit?: 'contain' | 'width';
  compact?: boolean;
  /** Names the page for screen readers: "the home page from the site dev page". */
  label: string;
  /** Shows the current stop on the page. */
  showStop?: boolean;
  /** A still image instead of a page, for this run's poster. */
  image?: string | null;
  /** Shown in the viewport while there is no page: the server's state. */
  children?: ReactNode;
  className?: string;
}
