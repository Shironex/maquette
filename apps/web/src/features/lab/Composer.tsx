import { useEffect, useRef, useState, type DragEvent } from 'react';
import {
  Button,
  Field,
  Icon,
  IconButton,
  NumberStepper,
  Select,
  TextArea,
  TextField,
  cn,
} from '@maquette/ui';
import { commands, type Aspect, type LabView } from '@/ipc/client';
import { fileUrl, registerMockFile } from '@/ipc/files';
import { IN_TAURI } from '@/ipc/runtime';
import { Menu } from '@/features/common/Menu';
import { useAppStore } from '@/state/app-store';
import { ASPECTS, MAX_IMAGES, MAX_REFS, coverImage, directionId, inProject } from '@/state/lab';
import { useLabStore } from '@/state/lab-store';

const IMAGE_FILE = /\.(png|jpe?g|webp)$/i;

/** The left panel: the brief for a new direction (or more of an earlier one). */
export function Composer({ view }: { view: LabView }) {
  const draft = useLabStore(s => s.draft);
  const setDraft = useLabStore(s => s.setDraft);
  const generate = useLabStore(s => s.generate);
  const busy = useLabStore(s => s.busy);
  const reuse = useLabStore(s => s.reuse);
  const newDraft = useLabStore(s => s.newDraft);
  const selected = useLabStore(s => s.selected);
  const codex = useAppStore(s => s.tools.reports.find(r => r.id === 'codex'));
  const existing = view.directions.find(d => d.id === directionId(draft.name));
  const ready = draft.name.trim() !== '' && draft.prompt.trim() !== '';
  const codexReady = !codex || codex.status === 'found';
  const n = draft.count;

  return (
    <section
      aria-label="New direction"
      className="flex w-[320px] flex-none flex-col gap-3.5 overflow-y-auto border-r border-line bg-panel px-[18px] pt-[18px] pb-4"
    >
      <div className="flex items-center justify-between gap-2">
        <h1 className="flex items-center gap-2 text-panel-title">
          <Icon name={existing ? 'images' : 'plus'} size={14} className="text-clay-text" />
          {existing ? 'Add to direction' : 'New direction'}
        </h1>
        {existing ? (
          <Button size="sm" variant="ghost" icon="plus" onClick={newDraft}>
            New
          </Button>
        ) : null}
      </div>
      <TextField
        label="Direction name"
        value={draft.name}
        maxLength={60}
        placeholder="Moon island porch"
        onChange={e => setDraft({ name: e.target.value })}
      />
      <TextArea
        label="Prompt"
        rows={6}
        value={draft.prompt}
        placeholder="What should the concepts show? Setting, mood, light, the props that matter."
        onChange={e => setDraft({ prompt: e.target.value })}
      />
      <References view={view} />
      <div className="grid grid-cols-2 gap-2.5">
        <Select<Aspect>
          label="Aspect"
          options={ASPECTS}
          value={draft.aspect}
          onChange={aspect => setDraft({ aspect })}
        />
        <Field id="lab-count" label="Images">
          <div className="[&>span]:h-[34px] [&>span]:w-full [&>span>span]:min-w-0 [&>span>span]:flex-1">
            <NumberStepper
              label="Images"
              value={n}
              min={1}
              max={MAX_IMAGES}
              onChange={count => setDraft({ count })}
            />
          </div>
        </Field>
      </div>
      {view.directions.length > 0 ? (
        <div className="flex flex-col gap-1.5 border-t border-line pt-3.5">
          <span className="mb-1 text-caption text-fg-mute uppercase">Earlier directions</span>
          <ul className="flex list-none flex-col">
            {view.directions.map(d => {
              const cover = coverImage(d);
              const url = cover ? fileUrl(inProject(view.projectDir, cover.file)) : null;
              return (
                <li key={d.id}>
                  <button
                    type="button"
                    aria-current={d.id === selected ? 'true' : undefined}
                    onClick={() => reuse(d.id)}
                    className={cn(
                      '-mx-1.5 flex w-[calc(100%+12px)] cursor-pointer items-center gap-2.5 rounded-control px-1.5 py-1 text-left hover:bg-hover',
                      d.id === selected && 'bg-hover'
                    )}
                  >
                    {url ? (
                      <img
                        src={url}
                        alt=""
                        className="h-7 w-11 flex-none rounded-[5px] object-cover"
                      />
                    ) : (
                      <span className="h-7 w-11 flex-none rounded-[5px] bg-raised" />
                    )}
                    <span className="min-w-0 flex-1 truncate">{d.name}</span>
                    <span className="text-meta text-fg-mute tabular-nums">
                      {d.images.length} {d.images.length === 1 ? 'image' : 'images'}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}
      <div className="mt-auto flex flex-col gap-2 pt-1">
        <Button
          variant="primary"
          size="lg"
          block
          iconRight="arrow-right"
          loading={busy === 'generate'}
          disabled={!ready || busy === 'generate' || !codexReady}
          title={ready ? undefined : 'Name the direction and write a prompt first'}
          onClick={() => void generate()}
        >
          {`Generate ${n} ${n === 1 ? 'concept' : 'concepts'}`}
        </Button>
        <span className={cn('text-center text-meta', codexReady ? 'text-fg-mute' : 'text-amber')}>
          {codexReady
            ? `Runs through Codex CLI, ${view.concurrency} at a time`
            : `Codex CLI is not ready. ${codex?.hint ?? 'Install it, then run codex login.'}`}
        </span>
      </div>
    </section>
  );
}

/** The reference chips: drop images, pick from refs/ or choose a file. */
function References({ view }: { view: LabView }) {
  const draft = useLabStore(s => s.draft);
  const addRefs = useLabStore(s => s.addRefs);
  const removeRef = useLabStore(s => s.removeRef);
  const [menu, setMenu] = useState(false);
  const [over, setOver] = useState(false);
  const zone = useRef<HTMLDivElement>(null);
  const full = draft.refs.length >= MAX_REFS;
  useNativeDrop(zone, setOver, paths => void addRefs(paths.slice(0, MAX_REFS - draft.refs.length)));

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setOver(false);
    // A plain browser (mock mode) gets File objects without paths.
    if (IN_TAURI) return;
    const paths: string[] = [];
    for (const file of Array.from(e.dataTransfer.files)) {
      if (!IMAGE_FILE.test(file.name)) continue;
      const path = `/dropped/${file.name}`;
      registerMockFile(path, URL.createObjectURL(file));
      paths.push(path);
    }
    void addRefs(paths);
  };

  const unused = view.refs.filter(r => !draft.refs.includes(r.file));
  const choose = async () => {
    setMenu(false);
    const path = await commands.dialogPickFile('Choose a reference image');
    if (path) await addRefs([path]);
  };

  return (
    <Field id="lab-refs" label="References" hint="Drop images here to guide the look.">
      <div
        ref={zone}
        id="lab-refs"
        onDragOver={e => {
          e.preventDefault();
          setOver(true);
        }}
        onDragLeave={() => setOver(false)}
        onDrop={onDrop}
        className={cn(
          'relative -m-1 grid grid-cols-4 gap-2 rounded-card p-1 transition-colors duration-120',
          over && 'bg-clay-soft outline-[1.5px] outline-clay outline-dashed'
        )}
      >
        {draft.refs.map(ref => (
          <RefChip
            key={ref}
            url={fileUrl(inProject(view.projectDir, ref))}
            name={ref.slice(ref.lastIndexOf('/') + 1)}
            onRemove={() => removeRef(ref)}
          />
        ))}
        {full ? null : (
          <div>
            <button
              type="button"
              aria-label="Add reference image"
              aria-haspopup="menu"
              aria-expanded={menu}
              onClick={() => setMenu(m => !m)}
              className="grid h-[62px] w-full cursor-pointer place-items-center rounded-card border border-dashed border-line-strong bg-transparent text-fg-mute hover:border-clay-line hover:text-fg"
            >
              <Icon name="plus" size={16} />
            </button>
            {menu ? (
              <Menu
                label="Add a reference"
                onClose={() => setMenu(false)}
                className="absolute top-full right-0 left-0 z-30 mt-1"
                header={
                  unused.length > 0 ? (
                    <div className="flex flex-col gap-1.5 p-1.5 pb-1">
                      <span className="text-caption text-fg-mute uppercase">From refs/</span>
                      <div className="grid grid-cols-4 gap-1.5">
                        {unused.map(r => {
                          const url = fileUrl(inProject(view.projectDir, r.file));
                          return (
                            <button
                              key={r.file}
                              type="button"
                              title={r.name}
                              onClick={() => {
                                setMenu(false);
                                void addRefs([r.file]);
                              }}
                              className="h-12 cursor-pointer overflow-hidden rounded-control bg-raised ring-clay hover:ring-2"
                            >
                              {url ? (
                                <img src={url} alt={r.name} className="size-full object-cover" />
                              ) : (
                                <span className="text-meta">{r.name}</span>
                              )}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  ) : (
                    <p className="px-2.5 py-2 text-meta text-fg-mute">
                      Nothing in refs/ yet. Send picks there, or choose a file.
                    </p>
                  )
                }
                items={[
                  {
                    id: 'file',
                    label: 'Choose a file',
                    icon: 'folder-open',
                    onSelect: () => void choose(),
                  },
                ]}
              />
            ) : null}
          </div>
        )}
      </div>
    </Field>
  );
}

function RefChip({
  url,
  name,
  onRemove,
}: {
  url: string | null;
  name: string;
  onRemove: () => void;
}) {
  return (
    <figure
      className="group relative m-0 h-[62px] overflow-hidden rounded-card bg-raised"
      title={name}
    >
      {url ? (
        <img src={url} alt={name} className="block size-full object-cover" />
      ) : (
        <span className="grid size-full place-items-center px-1 text-center text-meta text-fg-mute">
          {name}
        </span>
      )}
      <IconButton
        icon="x"
        label={`Remove ${name}`}
        size="sm"
        variant="secondary"
        onClick={onRemove}
        className="absolute top-1 right-1 size-6 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100"
      />
    </figure>
  );
}

/**
 * In the app, files dropped on the window arrive as a Tauri event with
 * their paths (the web view never sees them). Drops over `zone` count.
 */
function useNativeDrop(
  zone: React.RefObject<HTMLDivElement | null>,
  setOver: (over: boolean) => void,
  onPaths: (paths: string[]) => void
) {
  const latest = useRef(onPaths);
  latest.current = onPaths;
  useEffect(() => {
    if (!IN_TAURI) return;
    let stop: (() => void) | null = null;
    let cancelled = false;
    const inside = (x: number, y: number) => {
      const rect = zone.current?.getBoundingClientRect();
      if (!rect) return false;
      const ratio = window.devicePixelRatio || 1;
      const [px, py] = [x / ratio, y / ratio];
      // The whole composer column accepts drops, not just the chips.
      return px >= rect.left - 18 && px <= rect.right + 18 && py >= 0;
    };
    void import('@tauri-apps/api/webview').then(({ getCurrentWebview }) =>
      getCurrentWebview()
        .onDragDropEvent(event => {
          const p = event.payload;
          if (p.type === 'over') setOver(inside(p.position.x, p.position.y));
          else if (p.type === 'leave') setOver(false);
          else if (p.type === 'drop') {
            setOver(false);
            if (inside(p.position.x, p.position.y))
              latest.current(p.paths.filter(path => IMAGE_FILE.test(path)));
          }
        })
        .then(fn => {
          if (cancelled) fn();
          else stop = fn;
        })
    );
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [zone, setOver]);
}
