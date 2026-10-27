import { Button, HelpMark, Icon, IconButton, SegmentedControl, Tip, cn } from '@maquette/ui';
import { fileUrl } from '@/ipc/files';
import {
  ASPECTS,
  MAX_OBJECTS,
  MAX_OBJECT_CHARS,
  aspectCss,
  objectsDirectionName,
} from '@/state/lab';
import type { ObjectsDraft } from '@/state/use-lab-store';
import { ObjectChip } from './ObjectChip';
import { ObjectEditor } from './ObjectEditor';
import { useNotesTip, useObjectRefsSheet } from './ObjectRefsSheet.hooks';
import { KEYS } from '@/lib/keys';

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

export interface ObjectRefsSheetProps {
  objects: ObjectsDraft;
}

/** The dialog itself: the source concept, the object field and how the refs are made. */
export default function ObjectRefsSheet({ objects }: ObjectRefsSheetProps) {
  const {
    setObjects,
    close,
    busy,
    ref,
    input,
    text,
    setText,
    editing,
    setEditing,
    longest,
    over,
    pending,
    commit,
    remove,
    onKey,
    onPaste,
    submit,
    saveEdit,
  } = useObjectRefsSheet(objects);
  const [tipDismissed, dismissTip] = useNotesTip();

  const list = objects.objects;
  const full = list.length >= MAX_OBJECTS;
  const n = pending.length;
  const url = fileUrl(objects.sourcePath);
  const nearLimit = longest >= MAX_OBJECT_CHARS - 12;
  const showTip = list.length > 0 && editing === null && !tipDismissed;
  const showItems = SHOWS.map(line => (
    <li key={line} className="flex gap-1.5 text-meta text-fg-dim">
      <Icon name="check" size={13} className="mt-[3px] flex-none text-sage" />
      {line}
    </li>
  ));
  const chips = list.map(name => (
    <ObjectChip
      key={name}
      name={name}
      notes={objects.notes[name]}
      editing={editing === name}
      onToggle={() => setEditing(editing === name ? null : name)}
      onRemove={() => remove(name)}
    />
  ));

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
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={close} />
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
              <ul className="m-0 flex list-none flex-col gap-1 p-0">{showItems}</ul>
            </div>
          </figure>

          <div className="flex min-w-0 flex-1 flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <div className="flex items-baseline justify-between">
                <span className="flex items-center gap-1.5">
                  <label htmlFor="objects-input" className="font-medium">
                    Objects
                  </label>
                  <HelpMark
                    label="About objects"
                    tip="Click an object to add notes for Codex. Notes guide the image but never become the asset name."
                  />
                </span>
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
                {chips}
                <input
                  ref={input}
                  id="objects-input"
                  value={text}
                  disabled={full}
                  autoFocus
                  placeholder={
                    full
                      ? 'That is the most for one go'
                      : list.length
                        ? 'Add another'
                        : 'lantern post'
                  }
                  onChange={e => setText(e.target.value)}
                  onKeyDown={onKey}
                  onPaste={onPaste}
                  onBlur={() => text.trim() && commit(text)}
                  className="h-[26px] min-w-[140px] flex-1 border-0 bg-transparent px-1 text-fg outline-none placeholder:text-fg-mute"
                />
              </div>
              {editing !== null && list.includes(editing) ? (
                <ObjectEditor
                  key={editing}
                  name={editing}
                  notes={objects.notes[editing] ?? ''}
                  onSave={next => saveEdit(editing, next)}
                  onCancel={() => setEditing(null)}
                />
              ) : over || nearLimit ? (
                <p
                  className={cn('text-meta tabular-nums', over ? 'text-danger' : 'text-fg-mute')}
                  role={over ? 'alert' : undefined}
                >
                  {longest} of {MAX_OBJECT_CHARS} characters
                  {over
                    ? '. The name is the asset name: shorten it and put the rest in notes.'
                    : ''}
                </p>
              ) : (
                <p className="text-meta text-fg-mute">
                  A short name per object; it names the asset. Enter or a comma adds one, a pasted
                  list adds one per line.
                </p>
              )}
              {showTip ? (
                <Tip onDismiss={dismissTip}>
                  Tip: click an object to add notes, like materials or what to leave out.
                </Tip>
              ) : null}
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
            disabled={n === 0 || busy || over}
            disabledReason={over ? 'Shorten the name in the field' : 'Name at least one object'}
            tooltip="Draws each named object on its own as an image, with Codex"
            shortcut={KEYS.submit}
            onClick={() => void submit()}
          >
            {n === 0 ? 'Make object refs' : `Make ${n} object ${n === 1 ? 'ref' : 'refs'}`}
          </Button>
        </footer>
      </div>
    </div>
  );
}
