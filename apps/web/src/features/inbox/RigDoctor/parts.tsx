/** The small pieces every rig doctor layout shares. */
import { Icon, cn } from '@maquette/ui';
import type { ReactNode } from 'react';
import type { RigDefect, RigRecord, RigStat } from '@/ipc/client';
import { stepLines } from '@/state/rig';

/** A label and a value in the left column. */
export function Facts({ title, rows }: { title: string; rows: [string, ReactNode][] }) {
  const lines = rows.map(([label, value]) => (
    <div key={label} className="flex items-baseline justify-between gap-3">
      <dt className="text-fg-mute">{label}</dt>
      <dd className="m-0 min-w-0 truncate text-right font-mono text-fg-dim">{value}</dd>
    </div>
  ));
  return (
    <div>
      <h3 className="mb-2 text-caption tracking-[0.06em] text-fg-mute uppercase">{title}</h3>
      <dl className="m-0 flex flex-col gap-1.5 text-meta">{lines}</dl>
    </div>
  );
}

/** The doctor's six steps. */
export function Steps({ record }: { record: RigRecord }) {
  const lines = stepLines(record).map((line, i) => (
    <li
      key={line.step}
      aria-current={line.mark === 'on' ? 'step' : undefined}
      className={cn(
        'flex items-center gap-3 rounded-[9px] border border-transparent px-2.5 py-2',
        line.mark === 'on' && 'border-clay-line bg-clay-soft'
      )}
    >
      <span
        className={cn(
          'grid size-[22px] flex-none place-items-center rounded-full border border-line-strong text-meta text-fg-mute',
          line.mark === 'done' && 'border-sage/40 bg-sage-soft text-sage',
          line.mark === 'on' && 'border-clay-text text-clay-text'
        )}
      >
        {line.mark === 'done' ? <Icon name="check" size={12} /> : i + 1}
      </span>
      <span className="min-w-0">
        <span className="block leading-5 font-medium text-fg">{line.title}</span>
        <span
          className={cn('block text-meta text-fg-mute', line.mark === 'on' && 'text-clay-text')}
        >
          {line.detail}
        </span>
      </span>
    </li>
  ));
  return (
    <ol className="m-0 flex list-none flex-col gap-1 p-0" aria-label="Rig doctor steps">
      {lines}
    </ol>
  );
}

/** One number under the sheets; red when it is still bad. */
export function Stats({ stats }: { stats: RigStat[] }) {
  const tiles = stats.map(s => {
    const tone = s.bad ? 'text-clay-text' : s.before ? 'text-sage' : undefined;
    return (
      <div key={s.label} className="rounded-[9px] border border-line bg-raised px-2.5 py-2">
        <p className="m-0 min-h-8 text-meta text-fg-mute">{s.label}</p>
        <p className="m-0 font-mono text-emphasis text-fg">
          {s.before ? (
            <>
              {s.before} <span className="text-meta text-fg-mute">to</span>{' '}
            </>
          ) : null}
          <span className={tone}>{s.after}</span>
        </p>
      </div>
    );
  });
  return (
    <div className="grid flex-none grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-2.5">
      {tiles}
    </div>
  );
}

/** A numbered defect, by place. */
export function DefectCard({ defect, on }: { defect: RigDefect; on?: boolean }) {
  const yes = defect.kind === 'needs-yes';
  const tags = defect.tags.map(t => (
    <span key={t} className="rounded-full bg-hover px-2 py-0.5 text-fg-dim">
      {t}
    </span>
  ));
  return (
    <article
      className={cn(
        'flex gap-2.5 rounded-[10px] border border-line bg-raised p-3',
        on && 'border-clay-line'
      )}
    >
      <Pin number={defect.number} amber={yes} />
      <div className="min-w-0">
        <h4 className="m-0 leading-5 font-medium text-fg">{defect.title}</h4>
        <div className="my-1 flex flex-wrap gap-1 font-mono text-caption">
          <span className="inline-flex items-center gap-1 rounded-full bg-hover px-2 py-0.5 text-fg-dim">
            <Icon name="map-pin" size={10} />
            {defect.place}
          </span>
          {tags}
          {defect.kind === 'note' ? null : (
            <span
              className={cn(
                'rounded-full px-2 py-0.5',
                yes ? 'bg-amber-soft text-amber' : 'bg-sage-soft text-sage'
              )}
            >
              {yes ? 'Needs your yes' : 'Weights only'}
            </span>
          )}
        </div>
        <p className="m-0 text-meta text-fg-mute">{defect.detail}</p>
      </div>
    </article>
  );
}

/** The number on a defect and on the cells that show it. */
export function Pin({ number, amber }: { number: number; amber?: boolean }) {
  return (
    <span
      className={cn(
        'grid size-5 flex-none place-items-center rounded-full bg-clay text-caption font-semibold text-clay-ink',
        amber && 'bg-amber'
      )}
    >
      {number}
    </span>
  );
}

/** An image of a file on disk, or an empty tile while it does not exist. */
export function Tile({
  url,
  alt,
  flagged,
  dim,
  aspect = 'h-full',
}: {
  url: string | null;
  alt: string;
  flagged?: boolean;
  dim?: boolean;
  /** The tile's shape: a class that matches its picture, or filling its parent. */
  aspect?: string;
}) {
  return (
    <div
      className={cn(
        'relative overflow-hidden rounded-[8px] border border-line bg-studio',
        aspect,
        flagged && 'border-clay-text',
        dim && 'opacity-45'
      )}
    >
      {url ? (
        <img src={url} alt={alt} className="block h-full w-full object-contain" />
      ) : (
        <div aria-hidden className="h-full min-h-20 w-full" />
      )}
    </div>
  );
}

/** The bar under a layout: what it means on the left, its actions on the right. */
export function Footer({
  note,
  warning,
  children,
}: {
  note: ReactNode;
  /** Said instead of the note: something to look at first, which blocks nothing. */
  warning?: string | null;
  children: ReactNode;
}) {
  return (
    <footer className="flex flex-none items-center gap-3 border-t border-line px-5 py-3">
      <Icon
        name={warning ? 'warning' : 'info'}
        size={15}
        className={cn('flex-none', warning ? 'text-amber' : 'text-clay-text')}
      />
      <p className={cn('m-0 min-w-0 flex-1', warning ? 'text-amber' : 'text-fg-dim')}>
        {warning ?? note}
      </p>
      <div className="flex flex-none items-center gap-2">{children}</div>
    </footer>
  );
}

/** The bar and the log line of a run. */
export function RunProgress({ record }: { record: RigRecord }) {
  const p = record.progress;
  if (!p) return null;
  const share = p.total > 0 ? Math.round((p.done / p.total) * 100) : 0;
  return (
    <div className="rounded-[9px] border border-line bg-raised p-3">
      <div className="mb-1.5 flex items-baseline justify-between gap-3 text-meta">
        <span className="font-medium text-fg">{p.label}</span>
        <span className="font-mono text-fg-mute">
          {p.done} of {p.total}
        </span>
      </div>
      <div
        role="progressbar"
        aria-label={p.label}
        aria-valuenow={p.done}
        aria-valuemin={0}
        aria-valuemax={p.total}
        className="h-1.5 overflow-hidden rounded-full bg-hover"
      >
        <div className="h-full bg-clay" style={{ width: `${share}%` }} />
      </div>
    </div>
  );
}
