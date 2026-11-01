import { Suspense, type ReactNode } from 'react';
import { StageLoader } from '@/features/common/ViewLoader';
import { lazyAfterPaint } from '@/lib/after-paint';
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
  Loader,
} from '@maquette/ui';
import { pathLength, type PathTool } from '@/state/paths';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import { badge } from '../badge';
import { PathsPanel } from '../PathsPanel';
import { PointsTable } from '../PointsTable';
import { useWait, waitCount } from '@/lib/waiting';
import { loadingLabel } from '../loadingLabel';
import { newPath, usePathsEditor } from './PathsEditor.hooks';

const PathsCanvas = lazyAfterPaint(() => import('../PathsCanvas'));

const TOOLS: SegmentedOption<PathTool>[] = [
  { value: 'select', label: 'Select', icon: 'cursor-click', shortcut: 'V' },
  { value: 'add', label: 'Add points', icon: 'plus', shortcut: 'A' },
  {
    value: 'surface',
    label: 'Draw on surfaces',
    icon: 'note-pencil',
    shortcut: 'D',
    hint: 'Drag over a model to lay points along it. A drag off a model still orbits.',
  },
];

export interface PathsEditorProps {
  project: string;
  /** The Inspector's tabs, shown above the editor. */
  tabs: ReactNode;
}

/** Curves through the scene, drawn point by point in 3D, that the site lights as it scrolls. */
export default function PathsEditor({ project, tabs }: PathsEditorProps) {
  const { store, layout, frame, frameAll, handlers, loadError, ready, loaded } =
    usePathsEditor(project);
  const { view, draft, selection, error, loading } = store;
  const reading = !view || view.project !== project || !layout;
  const readEstimate = useWait(`paths:${project}`, reading && !error, Boolean(error));
  const modelsEstimate = useWait(
    `paths-models:${project}`,
    !reading && !ready && !loadError && (view?.models.length ?? 0) > 0,
    Boolean(loadError)
  );

  if (!view || view.project !== project || !layout) {
    return (
      <main className="flex min-h-0 flex-1 flex-col gap-3.5 px-6 py-[18px]" aria-busy={loading}>
        {tabs}
        {error ? (
          <div className="grid flex-1 place-items-center">
            <EmptyState
              icon="warning-circle"
              title="The paths could not be read"
              body={error}
              action={
                <Button
                  icon="arrow-clockwise"
                  tooltip="Loads the paths again"
                  onClick={() => void store.load(project)}
                >
                  Try again
                </Button>
              }
            />
          </div>
        ) : (
          <div className="grid flex-1 place-items-center">
            <Loader label="Reading the paths..." estimate={readEstimate} />
          </div>
        )}
      </main>
    );
  }

  const path = selection ? draft.paths.find(p => p.id === selection.path) : undefined;
  const point = selection?.point ?? null;
  const hint =
    store.tool === 'add' && path
      ? 'Click a model or the ground to add a point · drag to orbit'
      : store.tool === 'surface' && path
        ? 'Drag over a model to draw on it · click it for one point · drag off it to orbit'
        : 'Click a point or a path · drag a point to move it · F frames all';
  const loadingModels = !ready && !loadError && view.models.length > 0;
  const status = store.saving ? 'Saving' : store.saveError ? 'Not saved' : 'Saved to layout.json';

  return (
    <main className="flex min-h-0 flex-1">
      <section className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 py-[18px]">
        {tabs}
        <div className="flex flex-wrap items-center gap-2">
          <h1 className="text-panel-title">Scene paths</h1>
          <HelpMark
            label="About the paths keys"
            tip="V selects, A adds points and D draws on surfaces. A click with Add points puts a point on the model or the ground under it, after the picked point; a drag with Draw on surfaces lays a run of points along the model. Delete removes the picked point, F frames the scene and the magnet snaps to 5 cm."
            side="bottom"
          />
          <span className="text-meta text-fg-mute">{status}</span>
          <div className="ml-auto flex items-center gap-1.5">
            <SegmentedControl<PathTool>
              label="Tool"
              options={TOOLS}
              value={store.tool}
              onChange={store.setTool}
            />
            <IconButton
              icon="magnet"
              size="sm"
              variant="secondary"
              label={store.snap ? 'Snapping to 5 cm' : 'Snapping is off'}
              hint={store.snap ? undefined : 'Turn it on to place points in 5 cm steps'}
              aria-pressed={store.snap}
              className={cn(store.snap && 'border-clay-line text-clay-text')}
              onClick={() => store.setSnap(!store.snap)}
            />
            <IconButton
              icon="frame-corners"
              size="sm"
              variant="secondary"
              label="Frame the scene"
              shortcut="F"
              onClick={frameAll}
            />
          </div>
        </div>

        {store.saveError ? (
          <InlineBanner tone="danger" title="The paths were not saved" detail={store.saveError} />
        ) : null}

        <div className="relative min-h-[380px] flex-1 overflow-hidden rounded-window border border-line bg-[#141311]">
          <CanvasBoundary resetKey={project}>
            <Suspense fallback={<StageLoader />}>
              <PathsCanvas
                className="absolute! inset-0"
                models={view.models}
                layout={layout}
                paths={draft.paths}
                selection={selection}
                tool={path ? store.tool : 'select'}
                snap={store.snap}
                frame={frame}
                handlers={handlers}
              />
            </Suspense>
          </CanvasBoundary>
          {path ? (
            <span className={cn(badge, 'top-3 left-3')}>
              <Icon name="path" size={12} />
              {path.name} · {path.points.length} points · {pathLength(path).toFixed(1)} m
              {point !== null ? ` · point ${point + 1}` : ''}
            </span>
          ) : null}
          {draft.paths.length > 0 ? (
            <span className={cn(badge, 'right-3 bottom-3 text-fg-dim')}>{hint}</span>
          ) : null}
          {draft.paths.length === 0 ? (
            <div className="absolute inset-0 grid place-items-center bg-scrim">
              <EmptyState
                icon="path"
                title="No paths yet"
                body="A path is a curve through the scene the site draws as a tube and lights as the page scrolls: a cable, a pipe, a glowing trail."
                action={
                  <Button
                    icon="plus"
                    tooltip="Adds a path the site draws as a tube through the scene"
                    onClick={newPath}
                  >
                    New path
                  </Button>
                }
              />
            </div>
          ) : null}
          {loadingModels ? (
            <div className="pointer-events-none absolute inset-x-0 top-3 flex justify-center">
              <Loader
                label={loadingLabel}
                count={waitCount(loaded, view.models.length)}
                estimate={modelsEstimate}
              />
            </div>
          ) : null}
          {loadError ? (
            <span className="absolute inset-x-6 top-3 rounded-card bg-danger-soft px-3 py-2 text-center text-danger">
              {loadError}
            </span>
          ) : null}
        </div>
        {path ? <PointsTable path={path} models={view.models} /> : null}
      </section>
      <PathsPanel view={view} />
    </main>
  );
}
