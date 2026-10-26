import type { FrameBadge } from '../PageFrame';

export interface CompareRowView {
  label: string;
  value: string;
  change?: string;
  better?: boolean | null;
}

export interface CompareColumnView {
  id: string;
  title: string;
  /** "moon-island-site · shipped 3 days ago". */
  sub: string;
  /** The clay title and the "Ships next" mark of this run. */
  next?: boolean;
  src: string | null;
  image: string | null;
  address: string;
  badge: FrameBadge;
  /** What the column shows without a page or a still. */
  empty: string | null;
  rows: CompareRowView[];
}
