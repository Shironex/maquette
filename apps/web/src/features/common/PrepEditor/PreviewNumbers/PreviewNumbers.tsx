import { cn } from '@maquette/ui';
import type { FixupReport } from '@/ipc/client';
import { FIXUP_LABEL, reportRows, stepLine } from '@/state/prepare';

export interface PreviewNumbersProps {
  report: FixupReport;
}

/** A preview's numbers before and after, then what each fix-up did. */
export default function PreviewNumbers({ report }: PreviewNumbersProps) {
  const rows = reportRows(report).map(row => (
    <tr key={row.label} className="border-t border-line">
      <th scope="row" className="py-1.5 pr-2 text-left font-normal text-fg-dim">
        {row.label}
      </th>
      <td className="py-1.5 pl-2 text-right font-mono text-[11px] whitespace-nowrap text-fg-mute tabular-nums">
        {row.before}
      </td>
      <td
        className={cn(
          'py-1.5 pl-3 text-right font-mono text-[11px] whitespace-nowrap tabular-nums',
          row.changed ? 'text-sage' : 'text-fg'
        )}
      >
        {row.after}
      </td>
    </tr>
  ));
  const steps = report.steps.map(step => (
    <li key={step.op} className="flex gap-1.5">
      <span className="flex-none text-fg-dim">{FIXUP_LABEL[step.op]}:</span>
      <span className="min-w-0">{stepLine(step)}</span>
    </li>
  ));
  return (
    <div className="flex flex-col gap-2 rounded-card border border-line bg-app px-3 py-2">
      <table className="w-full text-[13px]" aria-label="Before and after the fix-ups">
        <thead>
          <tr className="text-meta text-fg-mute">
            <th scope="col" className="pb-1 text-left font-normal">
              <span className="sr-only">Number</span>
            </th>
            <th scope="col" className="pb-1 text-right font-normal">
              Before
            </th>
            <th scope="col" className="pb-1 text-right font-normal">
              After
            </th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
      {steps.length ? (
        <ol className="flex flex-col gap-0.5 border-t border-line pt-2 text-meta text-fg-mute">
          {steps}
        </ol>
      ) : null}
    </div>
  );
}
