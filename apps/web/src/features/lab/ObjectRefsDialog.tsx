/**
 * Make object refs: list the objects in a concept, and Codex makes one
 * clean reference image of each, with the concept attached, into a new
 * "<name> objects" direction. Opens from a Lab image and a Refs card.
 */
import { useRef, useState, type ClipboardEvent, type KeyboardEvent } from 'react';
import { Button, Icon, IconButton, SegmentedControl, cn } from '@maquette/ui';
import { fileUrl } from '@/ipc/files';
import { useTopEscape } from '@/lib/useTopEscape';
import { useAppStore } from '@/state/app-store';
import { ASPECTS, MAX_OBJECTS, addObjects, aspectCss, objectsDirectionName } from '@/state/lab';
import { useLabStore } from '@/state/lab-store';

const KINDS = [
  { value: 'prop' as const, label: 'Props' },
  { value: 'character' as const, label: 'Characters' },
];

const SHOWS = [
  'Only that object, centered, the whole of it in frame',
  'A front three-quarter view',
  'A plain light grey background, even soft light, no cast shadow',
  'No text, and the materials and style of the concept',
];

export function ObjectRefsDialog() {
  const objects = useLabStore(s => s.objects);
  if (!objects) return null;
  // Keyed on the source, so each open starts with an empty field.
  return <Dialog key={objects.source} />;
}

function Dialog() {
  const objects = useLabStore(s => s.objects);
  const setObjects = useLabStore(s => s.setObjects);
  const close = useLabStore(s => s.closeObjects);
  const make = useLabStore(s => s.makeObjects);
  const busy = useLabStore(s => s.busy === 'objects');
  const setView = useAppStore(s => s.setView);
  const [text, setText] = useState('');
  const ref = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  useTopEscape(ref, close);
  if (!objects) return null;

  const list = objects.objects;
  const full = list.length >= MAX_OBJECTS;
  const commit = (value: string) => {
    const next = addObjects(list, value);
    if (next.length !== list.length) setObjects({ objects: next });
    setText('');
  };
  const remove = (name: string) => setObjects({ objects: list.filter(o => o !== name) });
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if ((e.key === 'Enter' || e.key === ',') && text.trim()) {
      e.preventDefault();
      commit(text);
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey) && list.length > 0) {
      e.preventDefault();
      void submit();
    } else if (e.key === 'Backspace' && !text && list.length > 0) {
      remove(list[list.length - 1] ?? '');
    }
  };
  const onPaste = (e: ClipboardEvent<HTMLInputElement>) => {
    const pasted = e.clipboardData.getData('text');
    if (/[\n,;]/.test(pasted)) {
      e.preventDefault();
      commit(`${text}${pasted}`);
    }
  };
  // Text still in the field counts, so Make never drops the last object.
  const pending = addObjects(list, text);
  const submit = async () => {
    if (pending.length !== list.length) setObjects({ objects: pending });
    setText('');
    if (await make()) setView('lab');
  };
  const n = pending.length;
  const url = fileUrl(objects.sourcePath);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="objects-title"
        className="flex max-h-full w-[680px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="cube-focus" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="objects-title" className="truncate text-panel-title">
              Make object refs
            </h2>
            <p className="truncate text-meta text-fg-mute">
              From {objects.sourceName} · one Codex image per object, with this concept attached
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" onClick={close} />
        </header>

        <div className="flex min-h-0 flex-1 gap-5 overflow-y-auto px-5 py-4">
          <figure className="m-0 flex w-[220px] flex-none flex-col gap-2">
            <div className="overflow-hidden rounded-[10px] bg-studio shadow-[0_0_0_1px_var(--color-line)]">
              {url ? (
                <img
                  src={url}
                  alt={`${objects.sourceName}, the source concept`}
                  className="block w-full"
                />
              ) : null}
            </div>
            <figcaption className="truncate font-mono text-[11.5px] text-fg-mute">
              {objects.source}
            </figcaption>
            <div className="mt-1 flex flex-col gap-1.5 rounded-card border border-line bg-raised px-3 py-2.5">
              <p className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
                Each image shows
              </p>
              <ul className="m-0 flex list-none flex-col gap-1 p-0">
                {SHOWS.map(line => (
                  <li key={line} className="flex gap-1.5 text-meta text-fg-dim">
                    <Icon name="check" size={13} className="mt-[3px] flex-none text-sage" />
                    {line}
                  </li>
                ))}
              </ul>
            </div>
          </figure>

          <div className="flex min-w-0 flex-1 flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <div className="flex items-baseline justify-between">
                <label htmlFor="objects-input" className="font-medium">
                  Objects
                </label>
                <span className="text-meta text-fg-mute tabular-nums">
                  {n} of {MAX_OBJECTS}
                </span>
              </div>
              <div
                className={cn(
                  'flex min-h-[96px] cursor-text flex-wrap content-start items-start gap-1.5 rounded-control border border-line-strong bg-app p-2 focus-within:border-clay-line'
                )}
                onClick={() => input.current?.focus()}
              >
                {list.map(name => (
                  <span
                    key={name}
                    className="inline-flex h-[26px] items-center gap-1 rounded-pill border border-clay-line bg-clay-soft pr-1 pl-2.5 text-[12.5px] text-fg"
                  >
                    {name}
                    <button
                      type="button"
                      aria-label={`Remove ${name}`}
                      onClick={() => remove(name)}
                      className="grid size-[18px] cursor-pointer place-items-center rounded-full text-fg-mute hover:bg-panel hover:text-fg"
                    >
                      <Icon name="x" size={11} />
                    </button>
                  </span>
                ))}
                <input
                  ref={input}
                  id="objects-input"
                  value={text}
                  disabled={full}
                  autoFocus
                  placeholder={
                    full ? 'That is the most for one go' : list.length ? 'Add another' : 'desk lamp'
                  }
                  onChange={e => setText(e.target.value)}
                  onKeyDown={onKey}
                  onPaste={onPaste}
                  onBlur={() => text.trim() && commit(text)}
                  className="h-[26px] min-w-[140px] flex-1 border-0 bg-transparent px-1 text-fg outline-none placeholder:text-fg-mute"
                />
              </div>
              <p className="text-meta text-fg-mute">
                Name each object you want as its own model. Enter or a comma adds one; paste a list
                with one per line.
              </p>
            </div>

            <div className="flex items-center justify-between gap-3">
              <span className="text-fg-dim">They become</span>
              <SegmentedControl
                label="They become"
                tone="clay"
                options={KINDS}
                value={objects.kind}
                onChange={kind => setObjects({ kind })}
              />
            </div>
            <div className="flex items-center justify-between gap-3">
              <span className="text-fg-dim">Aspect</span>
              <SegmentedControl
                label="Aspect"
                options={ASPECTS}
                value={objects.aspect}
                onChange={aspect => setObjects({ aspect })}
              />
            </div>
            <div className="flex items-center gap-3 rounded-card border border-line bg-raised px-3 py-2.5">
              <span
                className="w-9 flex-none rounded-[4px] border border-line-strong bg-[#d9d7d3]"
                style={{ aspectRatio: aspectCss(objects.aspect) }}
              />
              <p className="text-meta text-fg-dim">
                Lands in{' '}
                <span className="font-medium text-fg">
                  {objectsDirectionName(objects.sourceName)}
                </span>
                , one image per object, each named after it. Pick the good ones, then send them to
                Refs or straight to 3D in one batch.
              </p>
            </div>
          </div>
        </div>

        <footer className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button variant="ghost" onClick={close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="sparkle"
            loading={busy}
            disabled={n === 0 || busy}
            title={n === 0 ? 'Name at least one object' : undefined}
            onClick={() => void submit()}
          >
            {n === 0 ? 'Make object refs' : `Make ${n} object ${n === 1 ? 'ref' : 'refs'}`}
          </Button>
        </footer>
      </div>
    </div>
  );
}
