import { Button } from '@maquette/ui';
import type { CompareRow } from '@/state/parts-compare';

export interface NumbersTableProps {
  label: string;
  leftLabel: string;
  rightLabel: string;
  rows: readonly CompareRow[];
}

/** Two sides' numbers, row by row, with a dot on the better one. */
export function NumbersTable({ label, leftLabel, rightLabel, rows }: NumbersTableProps) {
  const body = rows.map(row => (
    <tr key={row.label} className="border-b border-line last:border-b-0">
      <th scope="row" className="py-1.5 pr-2 pl-3 text-left font-medium text-fg-dim">
        {row.label}
      </th>
      <Cell text={row.left} better={row.better === 'left'} />
      <Cell text={row.right} better={row.better === 'right'} />
    </tr>
  ));
  return (
    <div className="flex-none overflow-hidden rounded-card border border-line bg-panel">
      <table aria-label={label} className="w-full table-fixed border-collapse text-[12px]">
        <colgroup>
          <col className="w-[180px]" />
          <col />
          <col />
        </colgroup>
        <thead>
          <tr className="border-b border-line text-[11px] tracking-[0.04em] text-fg-mute uppercase">
            <th className="py-1.5 pr-2 pl-3 text-left font-medium"> </th>
            <th className="px-2 py-1.5 text-left font-medium">{leftLabel}</th>
            <th className="px-2 py-1.5 text-left font-medium">{rightLabel}</th>
          </tr>
        </thead>
        <tbody>{body}</tbody>
      </table>
    </div>
  );
}

/** One side's number, with a dot when it is the better one. */
function Cell({ text, better }: { text: string; better: boolean }) {
  return (
    <td className="truncate px-2 py-1.5 font-mono text-fg tabular-nums" title={text}>
      {better ? (
        <span
          role="img"
          aria-label="better"
          className="mr-1.5 inline-block size-1.5 rounded-full bg-sage"
        />
      ) : null}
      {text}
    </td>
  );
}

interface KeepSide {
  label: string;
  onKeep: () => void;
}

export interface KeepRowProps {
  busy: boolean;
  tooltip: string;
  left: KeepSide;
  right: KeepSide;
}

/** What the dots mean and the two Keep buttons under the table. */
export function KeepRow({ busy, tooltip, left, right }: KeepRowProps) {
  return (
    <div className="flex items-center gap-2">
      <p className="flex-1 text-meta text-fg-mute">
        A dot marks the better number. Both versions stay in the asset either way.
      </p>
      <Button disabled={busy} tooltip={tooltip} onClick={left.onKeep}>
        {left.label}
      </Button>
      <Button variant="primary" disabled={busy} tooltip={tooltip} onClick={right.onKeep}>
        {right.label}
      </Button>
    </div>
  );
}
