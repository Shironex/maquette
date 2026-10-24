import { Suspense, lazy, useEffect, useState } from 'react';
import {
  BudgetMeter,
  Button,
  EmptyState,
  Icon,
  IconButton,
  InlineBanner,
  SegmentedControl,
  Tooltip,
  cn,
  type SegmentedOption,
} from '@maquette/ui';
import { useAppStore } from '@/state/app-store';
import { firstViewLine, firstViewMb, type Tool } from '@/state/layout';
import { useLayoutDirty, useLayoutStore } from '@/state/layout-store';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { LayoutPanel } from './LayoutPanel';

const LayoutCanvas = lazy(() => import('./LayoutCanvas'));

const TOOLS: SegmentedOption<Tool>[] = [
  { value: 'translate', label: 'Move', icon: 'arrows-out-cardinal' },
  { value: 'rotate', label: 'Turn', icon: 'arrow-clockwise' },
  { value: 'scale', label: 'Scale', icon: 'resize' },
];
const KEYS: Record<string, Tool> = { w: 'translate', e: 'rotate', r: 'scale' };

const badge =
  'absolute inline-flex h-[22px] items-center gap-[5px] rounded-[6px] bg-scrim px-2 text-[11.5px] font-medium text-fg';

function typing(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return Boolean(el?.closest('input, textarea, select, [contenteditable="true"]'));
}

/** The first view meter, Revert and Save, for the top bar. */
export function LayoutBarActions() {
  const view = useLayoutStore(s => s.view);
  const saving = useLayoutStore(s => s.saving);
  const save = useLayoutStore(s => s.save);
  const revert = useLayoutStore(s => s.revert);
  const dirty = useLayoutDirty();
  const fv = firstViewMb(view?.firstView);
  return (
    <>
      <Tooltip label={firstViewLine(view?.firstView)} wrap side="bottom">
        <span tabIndex={0} className="rounded-control">
          <BudgetMeter label="First view" value={fv.value} target={fv.target} unit="MB" />
        </span>
      </Tooltip>
      <Button icon="arrow-u-up-left" disabled={!dirty || saving} onClick={revert}>
        Revert
      </Button>
      <Button
        variant={dirty ? 'primary' : 'secondary'}
        icon="check"
        disabled={!view || !dirty || saving}
        onClick={() => void save()}
        title="Saves layout.json; the next pipeline run builds the scene and its posters from it"
      >
        {saving ? 'Saving…' : 'Save layout'}
      </Button>
    </>
  );
}

/** Where the models stand, the camera stops and the desk, edited in 3D. */
export function LayoutEditor({ project, tabs }: { project: string; tabs: React.ReactNode }) {
  const store = useLayoutStore();
  const setView = useAppStore(s => s.setView);
  const dirty = useLayoutDirty();
  const { view, draft, error, loading } = store;
  const [grid, setGrid] = useState(true);
  const [frame, setFrame] = useState(0);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    void store.load(project);
    setReady(false);
    setLoadError(null);
    // Only a new project loads again; agents' saves come as events.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target)) return;
      const s = useLayoutStore.getState();
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (mod && key === 'z') {
        e.preventDefault();
        if (e.shiftKey) s.redo();
        else s.undo();
      } else if (mod && key === 'y') {
        e.preventDefault();
        s.redo();
      } else if (mod && key === 's') {
        e.preventDefault();
        void s.save();
      } else if (!mod && KEYS[key]) {
        s.setTool(KEYS[key]);
      } else if (!mod && key === 'f') {
        setFrame(n => n + 1);
      } else if (key === 'escape') {
        if (s.preview) s.lookThrough(null);
        else s.select(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  if (!view || view.project !== project) {
    return (
      <main className="flex min-h-0 flex-1 flex-col gap-3.5 px-6 py-[18px]" aria-busy={loading}>
        {tabs}
        {error ? (
          <div className="grid flex-1 place-items-center">
            <EmptyState
              icon="warning-circle"
              title="The layout could not be read"
              body={error}
              action={
                <Button icon="arrow-clockwise" onClick={() => void store.load(project)}>
                  Try again
                </Button>
              }
            />
          </div>
        ) : null}
      </main>
    );
  }

  const previewStop = store.preview ? draft.stops.find(s => s.id === store.preview?.id) : null;
  const canUndo = store.history.past.length > 0;
  const canRedo = store.history.future.length > 0;

  return (
    <main className="flex min-h-0 flex-1">
      <section className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 py-[18px]">
        {tabs}
        <div className="flex flex-wrap items-center gap-2">
          <h1 className="text-panel-title">Scene layout</h1>
          <span className="text-meta text-fg-mute">
            {view.saved ? 'layout.json' : 'Not saved yet · models stand side by side'}
            {dirty ? ' · unsaved changes' : ''}
          </span>
          <div className="ml-auto flex items-center gap-1.5">
            <SegmentedControl
              label="Gizmo"
              options={TOOLS}
              value={store.selection?.kind === 'stop' ? 'translate' : store.tool}
              onChange={store.setTool}
            />
            <IconButton
              icon="magnet"
              size="sm"
              variant="secondary"
              label={store.snap ? 'Snapping to 5 cm and 15 degrees' : 'Snapping is off'}
              aria-pressed={store.snap}
              className={cn(store.snap && 'border-clay-line text-clay-text')}
              onClick={() => store.setSnap(!store.snap)}
            />
            <IconButton
              icon="grid-four"
              size="sm"
              variant="secondary"
              label={grid ? 'Hide the grid' : 'Show the grid'}
              aria-pressed={grid}
              className={cn(grid && 'border-clay-line text-clay-text')}
              onClick={() => setGrid(g => !g)}
            />
            <span className="mx-1 h-5 w-px bg-line" />
            <IconButton
              icon="arrow-u-up-left"
              size="sm"
              variant="secondary"
              label="Undo"
              disabled={!canUndo}
              onClick={store.undo}
            />
            <IconButton
              icon="arrow-u-up-right"
              size="sm"
              variant="secondary"
              label="Redo"
              disabled={!canRedo}
              onClick={store.redo}
            />
            <IconButton
              icon="frame-corners"
              size="sm"
              variant="secondary"
              label="Frame every model"
              onClick={() => {
                store.lookThrough(null);
                setFrame(n => n + 1);
              }}
            />
          </div>
        </div>

        {store.outside ? (
          <InlineBanner
            tone="warning"
            title="An agent changed the layout"
            detail="Your edits are not saved. Load their version to see it, or save yours over it."
            actions={
              <Button size="sm" onClick={() => void store.load(project)}>
                Load their version
              </Button>
            }
          />
        ) : null}

        {view.models.length === 0 ? (
          <div className="grid min-h-[340px] flex-1 place-items-center rounded-window border border-line">
            <EmptyState
              icon="cube-focus"
              title="No models to lay out yet"
              body="Accept a model from the Inbox. Every asset with a model shows up here to place."
              action={
                <Button icon="tray" onClick={() => setView('inbox')}>
                  Open the Inbox
                </Button>
              }
            />
          </div>
        ) : (
          <div className="relative min-h-[380px] flex-1 overflow-hidden rounded-window border border-line bg-[#141311]">
            <CanvasBoundary resetKey={project}>
              <Suspense fallback={null}>
                <LayoutCanvas
                  className="absolute! inset-0"
                  models={view.models}
                  layout={draft}
                  selection={store.selection}
                  tool={store.tool}
                  snap={store.snap}
                  grid={grid}
                  preview={store.preview}
                  frame={frame}
                  onSelect={store.select}
                  onBeginDrag={store.beginDrag}
                  onDragModel={store.dragModel}
                  onDragStop={(id, position) => store.dragStop(id, { position })}
                  onPose={store.setPose}
                  onUserMove={() => {
                    if (useLayoutStore.getState().preview) store.lookThrough(null);
                  }}
                  onError={setLoadError}
                  onReady={() => setReady(true)}
                />
              </Suspense>
            </CanvasBoundary>
            {previewStop ? <Letterbox /> : null}
            <span className={cn(badge, 'bottom-3 left-3')}>
              <Icon name="camera" size={12} />
              {previewStop
                ? `${previewStop.name} · ${previewStop.fov.toFixed(0)}°`
                : 'Free orbit · perspective'}
            </span>
            <span className={cn(badge, 'right-3 bottom-3 text-fg-dim')}>
              {store.selection?.kind === 'stop'
                ? 'Drag the camera to move the stop'
                : 'Click to select · drag to orbit · F frames all'}
            </span>
            {!ready && !loadError ? (
              <span className="absolute inset-0 grid place-items-center text-fg-mute">
                Loading {view.models.length} models…
              </span>
            ) : null}
            {loadError ? (
              <span className="absolute inset-x-6 top-3 rounded-card bg-danger-soft px-3 py-2 text-center text-danger">
                {loadError}
              </span>
            ) : null}
          </div>
        )}
      </section>
      <LayoutPanel view={view} dirty={dirty} />
    </main>
  );
}

/** Dims what falls outside the 16:9 frame the posters render. */
function Letterbox() {
  return (
    <div className="pointer-events-none absolute inset-0 grid place-items-center [container-type:size]">
      <div
        className="aspect-video rounded-[2px] shadow-[0_0_0_9999px_rgba(16,15,13,0.55)] outline outline-1 outline-clay-line"
        style={{ width: 'min(100cqw, calc(100cqh * 16 / 9))' }}
      />
    </div>
  );
}
