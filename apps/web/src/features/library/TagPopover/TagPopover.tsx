import { Icon, TextField, cn } from '@maquette/ui';
import type { LibraryItem } from '@/ipc/client';
import { useTagPopover } from './TagPopover.hooks';

export interface TagPopoverProps {
  /** The models the tags go on. */
  picked: readonly LibraryItem[];
  /** Every tag in the library, to offer as the field fills. */
  known: readonly string[];
  onClose: () => void;
  className?: string;
}

const HEADING = 'text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase';
const CHIP =
  'inline-flex h-6 cursor-pointer items-center gap-1 rounded-pill border px-2 font-mono text-[11.5px] disabled:cursor-default disabled:opacity-60';

/**
 * Tags for a selection: type and Enter puts a tag on every picked model, a
 * tag on all of them comes off with its x, and one on only some goes on
 * the rest with a click. It changes the library at once; nothing to save.
 */
export default function TagPopover({ picked, known, onClose, className }: TagPopoverProps) {
  const t = useTagPopover(picked, known, onClose);
  const n = picked.length;
  const label = `Tag ${n} ${n === 1 ? 'model' : 'models'}`;
  const rows = t.matches.map(m => (
    <button
      key={m.tag}
      type="button"
      disabled={t.busy}
      onClick={() => (m.on === n ? t.remove(m.tag) : t.add(m.tag))}
      className={cn(
        'flex h-8 cursor-pointer items-center justify-between rounded-control px-2 text-left hover:bg-hover',
        m.on === n && 'bg-clay-soft'
      )}
    >
      <span className="font-mono text-[12.5px]">{m.tag}</span>
      <span className="text-meta text-fg-mute">
        {m.on === n ? `on all ${n}, click to take off` : `on ${m.on} of ${n}`}
      </span>
    </button>
  ));
  const onAll = t.tags.all.map(tag => (
    <button
      key={tag}
      type="button"
      disabled={t.busy}
      aria-label={`Take ${tag} off all ${n}`}
      onClick={() => t.remove(tag)}
      className={cn(CHIP, 'border-clay-line bg-clay-soft text-fg')}
    >
      {tag}
      <Icon name="x" size={10} />
    </button>
  ));
  const onSome = t.tags.some.map(([tag, count]) => (
    <button
      key={tag}
      type="button"
      disabled={t.busy}
      aria-label={`Put ${tag} on all ${n}, it is on ${count}`}
      onClick={() => t.add(tag)}
      className={cn(CHIP, 'border-line text-fg-dim hover:border-line-strong')}
    >
      {tag} <span className="text-fg-mute">{count}</span>
    </button>
  ));
  return (
    <div
      ref={t.ref}
      role="dialog"
      aria-label={label}
      onKeyDown={t.onKeyDown}
      className={cn(
        'flex w-[300px] flex-col gap-2.5 rounded-card border border-line-strong bg-panel p-3 shadow-window',
        className
      )}
    >
      <div className="flex items-center gap-2">
        <b className="text-[13px]">{label}</b>
        <span className="flex-1" />
        <span className="text-meta text-fg-mute">Enter adds</span>
      </div>
      <TextField
        autoFocus
        icon="tag"
        aria-label="Tag"
        placeholder="Type a tag"
        value={t.typed}
        maxLength={32}
        onChange={e => t.setTyped(e.target.value)}
      />
      {rows.length || t.isNew ? (
        <div className="flex flex-col gap-0.5">
          {rows}
          {t.isNew ? (
            <button
              type="button"
              disabled={t.busy}
              onClick={() => t.add(t.text)}
              className="flex h-8 cursor-pointer items-center gap-1.5 rounded-control px-2 text-left hover:bg-hover"
            >
              <Icon name="plus" size={12} />
              Create <b className="font-mono">{t.text}</b>
            </button>
          ) : null}
        </div>
      ) : null}
      <div className="h-px bg-line" />
      <div className={HEADING}>On all {n}</div>
      {t.tags.all.length ? (
        <div className="flex flex-wrap gap-1">{onAll}</div>
      ) : (
        <p className="text-meta text-fg-mute">No tag is on every one yet.</p>
      )}
      {t.tags.some.length ? (
        <>
          <div className={HEADING}>On some</div>
          <div className="flex flex-wrap gap-1">{onSome}</div>
        </>
      ) : null}
    </div>
  );
}
