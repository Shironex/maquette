import {
  Button,
  InlineBanner,
  NumberField,
  SegmentedControl,
  Select,
  Toggle,
  cn,
} from '@maquette/ui';
import { RenderProblemCard } from '@/features/common/RenderProblemCard';
import { Sheet } from '@/features/common/Sheet';
import type { LoopFormat, RenderStyle } from '@/ipc/client';
import { LOOP_LABEL, STYLE_LABEL } from '@/state/renders';
import { ContactPreview } from '../ContactPreview';
import { RenderRows } from '../RenderRows';
import { useRenderSheet } from './RenderSheet.hooks';

const APPLY: readonly RenderStyle[] = [
  'clay',
  'toon',
  'blueprint',
  'shaded',
  'turntable',
  'contact-sheet',
];
const SCOPES = [
  { value: 'selected', label: 'Selected' },
  { value: 'all', label: 'Whole project' },
] as const;
const SIZES = ['1024', '2048', '4096'].map(v => ({ value: v, label: `${v} px stills` }));
const LOOPS = (['webp', 'gif', 'mp4'] as const).map(value => ({
  value,
  label: LOOP_LABEL[value],
}));

const CHIP = 'rounded-pill border px-2.5 py-0.5 text-meta transition-colors duration-120';

/**
 * Render a selection: one row per asset with its styles, a contact sheet of
 * all of them and what goes where. Closing it leaves a render running; the
 * status bar keeps count.
 */
export default function RenderSheet() {
  const s = useRenderSheet();
  if (!s.open) return null;
  const going = s.record?.state === 'running';
  const stopped = s.record?.state === 'stopped';
  const percent = s.progress ? (s.progress.done / Math.max(s.progress.total, 1)) * 100 : 0;
  const chips = APPLY.map(style => {
    const on = s.ticked.length > 0 && s.ticked.every(k => s.rows[k]?.includes(style));
    return (
      <button
        key={style}
        type="button"
        aria-pressed={on}
        onClick={() => s.applyToAll(style)}
        className={cn(
          CHIP,
          on ? 'border-clay-text/50 bg-clay-soft text-clay-text' : 'border-line-strong'
        )}
      >
        {STYLE_LABEL[style]}
      </button>
    );
  });
  const leftOut = s.leftOut.map(t => (
    <RenderProblemCard
      key={t.key}
      name={t.name}
      style="toon"
      problem={{
        kind: 'no-texture',
        message: `${t.name} has no texture. Clay works; toon would come out flat grey, so it is left out.`,
        webpBytes: null,
      }}
    />
  ));
  const problems = s.problems.map(p => (
    <RenderProblemCard
      key={p.file.file}
      name={p.name}
      style={p.file.style}
      bytes={p.file.bytes}
      problem={p.problem}
      actions={
        p.problem.kind === 'missing-tool'
          ? [{ label: 'Open Toolchain', icon: 'wrench', onClick: s.openToolchain }]
          : []
      }
    />
  ));
  const offersScope = s.isProject && !s.record && s.selection.length > 0;
  return (
    <Sheet
      id="render-batch"
      icon="image"
      title={s.title}
      meta={s.from}
      onClose={s.close}
      width={1000}
    >
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <div className="flex min-w-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          {offersScope ? (
            <SegmentedControl
              label="Which assets"
              options={SCOPES}
              value={s.scope}
              onChange={s.setScope}
            />
          ) : null}
          {s.error ? (
            <InlineBanner tone="danger" title="Cannot render this" detail={s.error} />
          ) : null}
          <RenderRows />
          {!s.record ? (
            <section
              aria-label="Apply to all"
              className="flex flex-col gap-2.5 rounded-card border border-line bg-raised p-3"
            >
              <h3 className="font-semibold">Apply to all {s.ticked.length}</h3>
              <div className="flex flex-wrap gap-1.5">{chips}</div>
              <div className="grid grid-cols-3 gap-2">
                <Select
                  label="Size"
                  value={String(s.settings.size)}
                  options={SIZES}
                  onChange={v => s.change({ size: Number(v) })}
                />
                <NumberField
                  label="Loops"
                  unit="px"
                  min={120}
                  max={1080}
                  value={s.settings.width}
                  onChange={width => s.change({ width })}
                />
                <Select<LoopFormat>
                  label="Loop format"
                  value={s.settings.format}
                  options={LOOPS}
                  onChange={format => s.change({ format })}
                />
              </div>
              <Toggle label="Replace earlier renders" checked={s.replace} onChange={s.setReplace} />
            </section>
          ) : null}
          {s.progress ? (
            <section aria-label="Progress" className="mt-auto flex flex-col gap-1.5">
              <div className="flex justify-between text-meta">
                <span className="text-fg-dim">
                  {s.runningLine ?? (stopped ? 'Stopped' : 'Finished')}
                </span>
                <span className="font-mono text-fg-dim">
                  {s.progress.done} of {s.progress.total} files
                  {s.estimate ? ` · ${s.estimate}` : ''}
                </span>
              </div>
              <div
                role="progressbar"
                aria-label="Files written"
                aria-valuemin={0}
                aria-valuemax={s.progress.total}
                aria-valuenow={s.progress.done}
                className="h-1.5 overflow-hidden rounded-pill bg-line"
              >
                <div className="h-full rounded-pill bg-amber" style={{ width: `${percent}%` }} />
              </div>
            </section>
          ) : null}
        </div>
        <aside className="flex w-[370px] flex-none flex-col gap-3 overflow-y-auto border-l border-line p-4">
          <ContactPreview />
          {leftOut}
          {problems}
        </aside>
      </div>
      <div className="flex items-center gap-2 border-t border-line px-4 py-3">
        <span className="flex-1 text-meta text-fg-mute">
          Renders run one at a time beside the pipeline. You can close this; the status bar keeps
          count.
        </span>
        {going ? (
          <Button icon="pause" onClick={() => void s.stop()}>
            Pause
          </Button>
        ) : null}
        {stopped ? (
          <Button icon="play" onClick={() => void s.resume()}>
            Go on
          </Button>
        ) : null}
        {s.record ? (
          <Button icon="folder" onClick={() => void s.reveal()}>
            Open the exports folder
          </Button>
        ) : (
          <Button
            variant="primary"
            icon="image"
            loading={s.busy}
            disabled={s.fileCount === 0}
            disabledReason="Pick at least one style for one asset"
            onClick={() => void s.start()}
          >
            Render {s.fileCount} {s.fileCount === 1 ? 'file' : 'files'}
          </Button>
        )}
      </div>
    </Sheet>
  );
}
