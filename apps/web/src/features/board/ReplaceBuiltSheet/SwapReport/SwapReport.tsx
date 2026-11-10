import { Icon, InlineBanner } from '@maquette/ui';
import type { ReplaceReport, SwapSide } from '@/ipc/client';
import {
  SETTING_GROUPS,
  boundsWords,
  modelNumbers,
  scaleWords,
  settingsOf,
  standingSize,
} from '@/state/built-replace';

export interface SwapReportProps {
  report: ReplaceReport;
}

const ROWS: readonly { label: string; cell: (side: SwapSide) => string | null }[] = [
  { label: 'File', cell: side => side.file },
  { label: 'Ships as', cell: side => side.shipsAs },
  { label: 'Model', cell: side => modelNumbers(side) || null },
  { label: 'Stands', cell: standingSize },
  { label: 'Real size', cell: scaleWords },
  { label: 'Bounds, m', cell: boundsWords },
];

/**
 * What a replace changes, before it does: the old and the built model side
 * by side (file names, triangles, bytes, how big each stands and its
 * bounds), what to check by eye, and every setting as it carries over,
 * comes along, is taken off or stays.
 */
export default function SwapReport({ report }: SwapReportProps) {
  const rows = ROWS.map(row => (
    <tr key={row.label} className="border-t border-line align-top">
      <th scope="row" className="py-1.5 pr-3 text-left font-normal text-fg-mute">
        {row.label}
      </th>
      <td className="py-1.5 pr-3 font-mono text-[11.5px] break-all text-fg-dim">
        {row.cell(report.old) ?? '·'}
      </td>
      <td className="py-1.5 font-mono text-[11.5px] break-all text-fg">
        {row.cell(report.new) ?? '·'}
      </td>
    </tr>
  ));
  const checks = [...report.warnings, ...report.unresolved].map(line => <li key={line}>{line}</li>);
  const groups = SETTING_GROUPS.map(group => {
    const lines = settingsOf(report, group.action);
    if (lines.length === 0) return null;
    const off = group.action === 'reset';
    const items = lines.map(line => (
      <li key={line.what} className="flex items-start gap-2">
        <Icon
          name={off ? 'minus' : 'check'}
          size={14}
          className={off ? 'mt-0.5 flex-none text-clay-text' : 'mt-0.5 flex-none text-sage'}
        />
        <span className="min-w-0">
          <b className="font-semibold text-fg">{line.what}</b>
          <span className="block text-fg-mute">{line.why}</span>
        </span>
      </li>
    ));
    return (
      <section key={group.action} aria-label={group.title} className="flex flex-col gap-1.5">
        <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
          {group.title} <span className="ml-1 tabular-nums">{lines.length}</span>
        </h3>
        <ul className="flex list-none flex-col gap-1.5 p-0 text-[12.5px]">{items}</ul>
      </section>
    );
  });

  return (
    <div className="flex flex-col gap-4">
      <table className="w-full table-fixed border-collapse text-[12.5px]">
        <caption className="sr-only">
          {report.name}: the old model and the built model that takes its place
        </caption>
        <colgroup>
          <col className="w-[84px]" />
          <col />
          <col />
        </colgroup>
        <thead>
          <tr className="text-left text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
            <td />
            <th scope="col" className="pb-1.5 font-semibold">
              Now, v{report.old.version}
            </th>
            <th scope="col" className="pb-1.5 font-semibold">
              Built, v{report.new.version}
            </th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
      <p className="text-[12.5px] text-fg-dim">
        {report.bakes
          ? 'The next run bakes its light like every other built piece.'
          : 'The next run leaves it out of the bake.'}{' '}
        Its parts: {report.parts.length ? report.parts.join(', ') : 'none read'}.
      </p>
      {checks.length ? (
        <InlineBanner
          tone="warning"
          title={`${checks.length} to check`}
          detail={<ul className="m-0 flex list-disc flex-col gap-1 pl-4">{checks}</ul>}
        />
      ) : null}
      {groups}
      <p className="text-[12px] text-fg-mute">{report.note}</p>
    </div>
  );
}
