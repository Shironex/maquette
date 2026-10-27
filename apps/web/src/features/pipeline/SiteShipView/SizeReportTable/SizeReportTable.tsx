import { Button } from '@maquette/ui';
import type { SizeReport } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { keptShare, useCopyReport } from './SizeReportTable.hooks';

export interface SizeReportTableProps {
  report: SizeReport;
}

const bytes = (n: number | null) => (n ? formatBytes(n) : '·');
const tris = (n: number | null) => (n === null ? '·' : formatK(n));

const FORMAT_WORD = { ktx2: 'KTX2', webp: 'WebP', avif: 'AVIF' } as const;

/** Each model's source bytes and triangles against what ships. */
export default function SizeReportTable({ report }: SizeReportTableProps) {
  const copyCsv = useCopyReport(report.csv, 'The size report as CSV');
  const copyMarkdown = useCopyReport(report.markdown, 'The size report as Markdown');
  const { totals } = report;
  const empty = report.rows.length === 0;
  const rows = report.rows.map(r => (
    <tr key={r.asset} className="border-t border-line">
      <td className="py-1.5 pl-3">
        <span className="block truncate">{r.name}</span>
        <span className="block truncate font-mono text-[11px] text-fg-mute" title={r.shippedFile}>
          v{r.version} · {r.shippedFile.split('/').pop()}
        </span>
      </td>
      <td className="px-2 py-1.5 text-right font-mono tabular-nums">{bytes(r.sourceBytes)}</td>
      <td className="px-2 py-1.5 text-right font-mono tabular-nums">{bytes(r.shippedBytes)}</td>
      <td className="px-2 py-1.5 text-right font-mono text-fg-mute tabular-nums">
        {keptShare(r.sourceBytes, r.shippedBytes)}
      </td>
      <td className="px-2 py-1.5 text-right font-mono tabular-nums">{tris(r.sourceTriangles)}</td>
      <td className="py-1.5 pr-3 pl-2 text-right font-mono tabular-nums">
        {tris(r.shippedTriangles)}
      </td>
    </tr>
  ));
  return (
    <section aria-labelledby="size-report-title" className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <h2
          id="size-report-title"
          className="flex-1 text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase"
        >
          Size report · {FORMAT_WORD[report.textures]} textures
        </h2>
        <Button
          size="sm"
          variant="ghost"
          icon="copy"
          disabled={empty}
          disabledReason="No models to report yet"
          tooltip="Copies one row per model, bytes and triangles as plain numbers"
          onClick={copyCsv}
        >
          Copy CSV
        </Button>
        <Button
          size="sm"
          variant="ghost"
          icon="copy"
          disabled={empty}
          disabledReason="No models to report yet"
          tooltip="Copies a table for a pull request or a note, sizes in KB"
          onClick={copyMarkdown}
        >
          Copy Markdown
        </Button>
      </div>
      <div className="rounded-card border border-line bg-panel">
        <table className="w-full table-fixed border-collapse text-[12px]">
          <colgroup>
            <col />
            <col className="w-[96px]" />
            <col className="w-[96px]" />
            <col className="w-[64px]" />
            <col className="w-[96px]" />
            <col className="w-[104px]" />
          </colgroup>
          <thead>
            <tr className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
              <th className="py-1.5 pl-3 text-left font-semibold">Model</th>
              <th className="px-2 py-1.5 text-right font-semibold">Source</th>
              <th className="px-2 py-1.5 text-right font-semibold">Shipped</th>
              <th className="px-2 py-1.5 text-right font-semibold">Kept</th>
              <th className="px-2 py-1.5 text-right font-semibold">Source tris</th>
              <th className="py-1.5 pr-3 pl-2 text-right font-semibold">Shipped tris</th>
            </tr>
          </thead>
          <tbody>
            {empty ? (
              <tr className="border-t border-line">
                <td colSpan={6} className="py-2 pl-3 text-fg-mute">
                  No models yet. Each asset with a model gets a row.
                </td>
              </tr>
            ) : (
              rows
            )}
          </tbody>
          {empty ? null : (
            <tfoot>
              <tr className="border-t border-line bg-raised font-semibold">
                <td className="py-1.5 pl-3">All {totals.models} measured</td>
                <td className="px-2 py-1.5 text-right font-mono tabular-nums">
                  {bytes(totals.sourceBytes)}
                </td>
                <td className="px-2 py-1.5 text-right font-mono tabular-nums">
                  {bytes(totals.shippedBytes)}
                </td>
                <td className="px-2 py-1.5 text-right font-mono text-fg-mute tabular-nums">
                  {keptShare(totals.sourceBytes, totals.shippedBytes)}
                </td>
                <td className="px-2 py-1.5 text-right font-mono tabular-nums">
                  {tris(totals.sourceTriangles)}
                </td>
                <td className="py-1.5 pr-3 pl-2 text-right font-mono tabular-nums">
                  {tris(totals.shippedTriangles)}
                </td>
              </tr>
            </tfoot>
          )}
        </table>
      </div>
    </section>
  );
}
