import type { SortKey } from '@/state/board-list';

export interface ListColumn {
  key: SortKey;
  label: string;
  /** A tooltip line for a heading that needs a word more. */
  hint?: string;
  /** Numbers read right aligned. */
  numeric?: boolean;
  /** The column's width; the name takes what is left. */
  width?: string;
  /** Classes that drop the column on a narrow window so the name keeps room. */
  narrow?: string;
}

/** Below the wide breakpoint the least needed columns step aside, so Name and the row menu stay. */
const NO_WIDE = 'max-[1279px]:hidden';
const NO_MID = 'max-[1023px]:hidden';

/** The sortable columns of the Board's list, in order. */
export const LIST_COLUMNS: readonly ListColumn[] = [
  { key: 'name', label: 'Name' },
  { key: 'kind', label: 'Kind', width: 'w-[44px] min-[1280px]:w-[96px]' },
  { key: 'stage', label: 'Stage', width: 'w-[112px]' },
  { key: 'version', label: 'Version', width: 'w-[76px]', narrow: NO_WIDE },
  { key: 'tris', label: 'Tris', numeric: true, width: 'w-[76px]', narrow: NO_WIDE },
  {
    key: 'size',
    label: 'Size',
    hint: 'The shipped build once Compress ran, else the model file (marked source)',
    numeric: true,
    width: 'w-[104px]',
    narrow: NO_WIDE,
  },
  { key: 'status', label: 'Status', width: 'w-[188px]' },
  { key: 'changed', label: 'Last change', width: 'w-[112px]', narrow: NO_MID },
];
