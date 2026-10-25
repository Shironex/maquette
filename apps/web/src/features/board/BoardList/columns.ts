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
}

/** The sortable columns of the Board's list, in order. */
export const LIST_COLUMNS: readonly ListColumn[] = [
  { key: 'name', label: 'Name' },
  { key: 'kind', label: 'Kind', width: 'w-[96px]' },
  { key: 'stage', label: 'Stage', width: 'w-[112px]' },
  { key: 'version', label: 'Version', width: 'w-[76px]' },
  { key: 'tris', label: 'Tris', numeric: true, width: 'w-[76px]' },
  {
    key: 'size',
    label: 'Size',
    hint: 'The shipped build once Compress ran, else the model file (marked source)',
    numeric: true,
    width: 'w-[104px]',
  },
  { key: 'status', label: 'Status', width: 'w-[188px]' },
  { key: 'changed', label: 'Last change', width: 'w-[112px]' },
];
