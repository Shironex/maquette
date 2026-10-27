import { Suspense, lazy, type ReactNode } from 'react';
import {
  Button,
  EmptyState,
  HelpMark,
  Icon,
  IconButton,
  InlineBanner,
  SegmentedControl,
  cn,
  type SegmentedOption,
} from '@maquette/ui';
import type { Tool } from '@/state/layout';
import { useLayoutStore } from '@/state/use-layout-store';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { badge } from '../badge';
import { LayoutPanel } from '../LayoutPanel';
import { Letterbox } from '../Letterbox';
import { useLayoutEditor } from './LayoutEditor.hooks';
import { KEYS } from '@/lib/keys';

const LayoutCanvas = lazy(() => import('../LayoutCanvas'));

const TOOLS: SegmentedOption<Tool>[] = [
  { value: 'translate', label: 'Move', icon: 'arrows-out-cardinal', shortcut: 'W' },
  { value: 'rotate', label: 'Turn', icon: 'arrow-clockwise', shortcut: 'E' },
  { value: 'scale', label: 'Scale', icon: 'resize', shortcut: 'R' },
];

export interface LayoutEditorProps {
  project: string;
  /** The Inspector's tabs, shown above the editor. */
  tabs: ReactNode;
}

/** Where the models stand, the camera stops and the desk, edited in 3D. */
export default function LayoutEditor({ project, tabs }: LayoutEditorProps) {
  const {
    store,
    setView,
    dirty,
    grid,
    toggleGrid,
    frame,
    frameAll,
    loadError,
    setLoadError,
    ready,
    setReady,
  } = useLayoutEditor(project);
  const { view, draft, error, loading } = store;

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
                <Button
                  icon="arrow-clockwise"
                  tooltip="Loads the layout again"
                  onClick={() => void store.load(project)}
                >
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
          <HelpMark
            label="About the layout keys"
            tip="W, E and R switch between move, turn and scale. F frames every model, Escape clears the selection, and the magnet snaps to 5 cm and 15 degrees."
            side="bottom"
          />
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
              hint={
                store.snap ? undefined : 'Turn it on to move in 5 cm and turn in 15 degree steps'
              }
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
              onClick={toggleGrid}
            />
            <span className="mx-1 h-5 w-px bg-line" />
            <IconButton
              icon="arrow-u-up-left"
              size="sm"
              variant="secondary"
              label="Undo"
              shortcut={KEYS.undo}
              disabled={!canUndo}
              disabledReason="Nothing to undo yet"
              onClick={store.undo}
            />
            <IconButton
              icon="arrow-u-up-right"
              size="sm"
              variant="secondary"
              label="Redo"
              shortcut={KEYS.redo}
              disabled={!canRedo}
              disabledReason="Nothing to redo"
              onClick={store.redo}
            />
            <IconButton
              icon="frame-corners"
              size="sm"
              variant="secondary"
              label="Frame every model"
              shortcut="F"
              onClick={() => {
                store.lookThrough(null);
                frameAll();
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
              <Button
                size="sm"
                tooltip="Replaces your unsaved edits with the layout as it is on disk"
                onClick={() => void store.load(project)}
              >
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
                <Button
                  icon="tray"
                  tooltip="Goes to the Inbox, where models arrive"
                  onClick={() => setView('inbox')}
                >
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
                  handlers={{
                    onSelect: store.select,
                    onBeginDrag: store.beginDrag,
                    onDragModel: store.dragModel,
                    onDragStop: (id, position) => store.dragStop(id, { position }),
                    onPose: store.setPose,
                    onUserMove: () => {
                      if (useLayoutStore.getState().preview) store.lookThrough(null);
                    },
                    onError: setLoadError,
                    onReady: () => setReady(true),
                  }}
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
