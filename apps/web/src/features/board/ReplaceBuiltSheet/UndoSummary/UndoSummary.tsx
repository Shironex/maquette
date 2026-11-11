import { Icon, InlineBanner } from '@maquette/ui';
import type { UndoReport } from '@/ipc/client';
import { modelNumbers } from '@/state/built-replace';

export interface UndoSummaryProps {
  report: UndoReport;
}

/**
 * What bringing the old model back changes, before it does: the model that
 * is current again, the versions that leave the asset (their files go to
 * the workspace trash, never deleted), the settings that come back with it
 * and the built asset's own card.
 */
export default function UndoSummary({ report }: UndoSummaryProps) {
  const off = report.takenOff.map(v => (
    <li key={v.version} className="flex items-start gap-2">
      <Icon name="minus" size={14} className="mt-0.5 flex-none text-clay-text" />
      <span className="min-w-0">
        <b className="font-semibold text-fg">v{v.version}</b>{' '}
        <span className="font-mono text-[11.5px] break-all text-fg-dim">{v.file}</span>
        <span className="block text-fg-mute">{modelNumbers(v)}</span>
      </span>
    </li>
  ));
  const back = report.settings.map(line => (
    <li key={line} className="flex items-start gap-2">
      <Icon name="check" size={14} className="mt-0.5 flex-none text-sage" />
      <span>{line}</span>
    </li>
  ));
  const notes = report.warnings.map(line => <li key={line}>{line}</li>);
  const heading = 'text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase';

  return (
    <div className="flex flex-col gap-4 text-[12.5px]">
      <section aria-label="Current again" className="flex flex-col gap-1.5">
        <h3 className={heading}>Current again</h3>
        <p className="text-fg">
          <b className="font-semibold">v{report.back.version}</b>{' '}
          <span className="font-mono text-[11.5px] break-all text-fg-dim">{report.back.file}</span>
          <span className="block text-fg-mute">{modelNumbers(report.back)}</span>
        </p>
      </section>
      <section aria-label="Leaves the asset" className="flex flex-col gap-1.5">
        <h3 className={heading}>
          Leaves the asset <span className="ml-1 tabular-nums">{report.takenOff.length}</span>
        </h3>
        <ul className="flex list-none flex-col gap-1.5 p-0">{off}</ul>
        <p className="text-fg-mute">
          Their files move to the workspace trash. Nothing is deleted, and {report.script} stops
          building this asset.
        </p>
      </section>
      {back.length ? (
        <section aria-label="Comes back" className="flex flex-col gap-1.5">
          <h3 className={heading}>
            Comes back <span className="ml-1 tabular-nums">{back.length}</span>
          </h3>
          <ul className="flex list-none flex-col gap-1.5 p-0 text-fg-dim">{back}</ul>
        </section>
      ) : null}
      {report.restores ? (
        <p className="text-fg-dim">
          {report.restores} comes back on the Board as a built asset of its own, from the asset
          trash.
        </p>
      ) : null}
      {report.warnings.length ? (
        <InlineBanner
          tone="warning"
          title={`${report.warnings.length} to know`}
          detail={<ul className="m-0 flex list-disc flex-col gap-1 pl-4">{notes}</ul>}
        />
      ) : null}
    </div>
  );
}
