import { Checkbox, cn } from '@maquette/ui';
import type { RenderItem, RenderStyle, RenderTarget } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { rowState, STYLE_LABEL, STYLES, type RowTone } from '@/state/renders';
import { useRenderRows } from './RenderRows.hooks';

const TONE: Record<RowTone, string> = {
  done: 'bg-sage-soft text-sage',
  running: 'bg-amber-soft text-amber',
  queued: 'bg-raised text-fg-mute',
  warn: 'bg-amber-soft text-amber',
  failed: 'bg-danger-soft text-danger',
};
const CHIP = 'rounded-pill border border-line-strong px-2 text-meta';
const HEAD = 'py-1 font-medium';

function Thumb({ file }: { file: string | null }) {
  const url = fileUrl(file);
  return (
    <span className="size-8 flex-none overflow-hidden rounded-control border border-line bg-raised">
      {url ? <img src={url} alt="" className="size-full object-cover" /> : null}
    </span>
  );
}

function RunningRow({ item }: { item: RenderItem }) {
  const state = rowState(item);
  const chips = item.files.map(f => (
    <span key={f.style} className={CHIP}>
      {STYLE_LABEL[f.style]}
    </span>
  ));
  return (
    <tr className="border-t border-line">
      <td className="py-1.5 pr-2">
        <span className="flex items-center gap-2 font-medium">
          <Thumb file={item.target.thumbnail} />
          {item.target.name}
        </span>
      </td>
      <td className="py-1.5 pr-2">
        <span className="flex flex-wrap gap-1">{chips}</span>
      </td>
      <td className="py-1.5 whitespace-nowrap">
        <span className={cn('rounded-pill px-2 py-0.5 text-meta', TONE[state.tone])}>
          {state.label}
        </span>
      </td>
    </tr>
  );
}

interface PickRowProps {
  target: RenderTarget;
  styles: readonly RenderStyle[];
  on: boolean;
  onTick: (on: boolean) => void;
  onStyle: (style: RenderStyle) => void;
}

function PickRow({ target, styles, on, onTick, onStyle }: PickRowProps) {
  const chips = styles.map(style => (
    <button
      key={style}
      type="button"
      title="Takes this style off"
      aria-label={`Take ${STYLE_LABEL[style]} off ${target.name}`}
      onClick={() => onStyle(style)}
      className={cn(CHIP, 'hover:border-danger/50')}
    >
      {STYLE_LABEL[style]}
    </button>
  ));
  const left = STYLES.filter(x => !styles.includes(x.id)).map(x => (
    <option key={x.id} value={x.id}>
      {x.label}
    </option>
  ));
  return (
    <tr className="border-t border-line">
      <td className="py-1.5">
        <Checkbox
          label={<span className="sr-only">Render {target.name}</span>}
          checked={on}
          onChange={onTick}
        />
      </td>
      <td className="py-1.5 pr-2">
        <span className="flex items-center gap-2 font-medium">
          <Thumb file={target.thumbnail} />
          {target.name}
        </span>
      </td>
      <td className="py-1.5">
        <span className="flex flex-wrap items-center gap-1">
          {chips}
          <select
            aria-label={`Add a style to ${target.name}`}
            value=""
            onChange={e => onStyle(e.target.value as RenderStyle)}
            className="rounded-pill border border-dashed border-line-strong bg-transparent px-1.5 text-meta text-fg-mute"
          >
            <option value="">+ Style</option>
            {left}
          </select>
        </span>
      </td>
    </tr>
  );
}

/** One row per asset: tick, thumbnail, name, styles, and where it stands. */
export default function RenderRows() {
  const r = useRenderRows();
  if (r.record) {
    const running = r.record.items.map(item => <RunningRow key={item.target.key} item={item} />);
    return (
      <table className="w-full text-left">
        <thead className="text-meta tracking-wide text-fg-mute uppercase">
          <tr>
            <th className={HEAD}>Asset</th>
            <th className={HEAD}>Styles</th>
            <th className={HEAD}>State</th>
          </tr>
        </thead>
        <tbody>{running}</tbody>
      </table>
    );
  }
  if (r.plan && r.shown.length === 0) {
    return (
      <p className="rounded-card border border-dashed border-line-strong p-4 text-fg-mute">
        Nothing in this selection has a model yet. Render styles start from a model; Whole project
        lists the assets that have one.
      </p>
    );
  }
  const picks = r.shown.map(t => (
    <PickRow
      key={t.key}
      target={t}
      styles={r.rows[t.key] ?? []}
      on={r.ticked.includes(t.key)}
      onTick={on => r.tick(t.key, on)}
      onStyle={style => r.toggleStyle(t.key, style)}
    />
  ));
  return (
    <table className="w-full text-left">
      <thead className="text-meta tracking-wide text-fg-mute uppercase">
        <tr>
          <th className="w-7" />
          <th className={HEAD}>Asset</th>
          <th className={HEAD}>Styles</th>
        </tr>
      </thead>
      <tbody>{picks}</tbody>
    </table>
  );
}
