import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Button,
  EmptyState,
  Icon,
  IconButton,
  SegmentedControl,
  StageStepper,
  Tabs,
  TopBar,
  cn,
  type SegmentedOption,
} from '@maquette/ui';
import type { CompareSide, CompressPreset, InspectView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { Menu, type MenuItem } from '@/features/common/Menu';
import { useAppStore } from '@/state/use-app-store';
import { projectFile } from '@/state/board';
import {
  modelForBudget,
  num,
  pickSides,
  sideModels,
  stopsHelp,
  stopsOf,
  vec3,
  versusLabel,
} from '@/state/inspector';
import {
  useInspectorStore,
  type InspectMode,
  type InspectorTab,
} from '@/state/use-inspector-store';
import { stageViews } from '@/state/pipeline';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { SceneBudgetMeter } from '@/features/common/SceneBudgetMeter';
import { CanvasBoundary } from '@/features/viewer/CanvasBoundary';
import type { CompareModel } from './CompareCanvas';
import { FrameTime } from './FrameTime';
import { LayoutBarActions, LayoutEditor } from './LayoutEditor';
import { ModelPanel } from './ModelPanel';

const CompareCanvas = lazy(() => import('./CompareCanvas'));

const MODES: SegmentedOption<InspectMode>[] = [
  { value: 'shaded', label: 'Shaded' },
  { value: 'wireframe', label: 'Wireframe' },
  { value: 'lightmap', label: 'Lightmap' },
];

const badge =
  'absolute inline-flex h-[22px] items-center gap-[5px] rounded-[6px] bg-scrim px-2 text-[11.5px] font-medium text-fg';

const TABS = [
  { id: 'compare', label: 'Compare' },
  { id: 'layout', label: 'Layout' },
] as const;

/** Compare versions, or lay out the scene. */
function InspectorTabs() {
  const tab = useInspectorStore(s => s.tab);
  const setTab = useInspectorStore(s => s.setTab);
  return (
    <Tabs
      tabs={TABS}
      value={tab}
      onChange={id => setTab(id as InspectorTab)}
      className="-mt-1 flex-none"
    />
  );
}

function Shell({ children, right }: { children: React.ReactNode; right?: React.ReactNode }) {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const latest = usePipelineStore(s => s.view?.runs[0]);
  return (
    <>
      <TopBar
        lead={
          project ? (
            <StageStepper stages={stageViews(project.assets ?? [], latest)} />
          ) : (
            <h1 className="text-panel-title">Inspector</h1>
          )
        }
        right={right}
      />
      {children}
    </>
  );
}

export function InspectorView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const { view, loading, error, load, tab } = useInspectorStore();
  const projectId = project?.id ?? null;
  const lastRun = usePipelineStore(s => s.view?.runs[0]?.finishedAtMs ?? null);

  useEffect(() => {
    if (projectId) void load(projectId);
    // A finished run changes the shipped models, so it reloads too.
  }, [projectId, lastRun, load]);

  if (project && tab === 'layout') {
    return (
      <Shell right={<LayoutBarActions />}>
        <LayoutEditor project={project.id} tabs={<InspectorTabs />} />
      </Shell>
    );
  }
  if (!project) {
    return (
      <Shell>
        <main className="grid flex-1 place-items-center">
          <EmptyState
            icon="cube-focus"
            title="No project yet"
            body="Create a project in Settings first."
          />
        </main>
      </Shell>
    );
  }
  if (!view || view.project !== project.id) {
    return (
      <Shell>
        <main className="flex flex-1 flex-col px-6 py-[18px]" aria-busy={loading}>
          <InspectorTabs />
          {error ? (
            <div className="grid flex-1 place-items-center">
              <EmptyState
                icon="warning-circle"
                title="The models could not be read"
                body={error}
                action={
                  <Button icon="arrow-clockwise" onClick={() => void load(project.id)}>
                    Try again
                  </Button>
                }
              />
            </div>
          ) : null}
        </main>
      </Shell>
    );
  }
  return <Inspector view={view} />;
}

function Inspector({ view }: { view: InspectView }) {
  const workspace = useAppStore(s => s.workspace);
  const setView = useAppStore(s => s.setView);
  const store = useInspectorStore();
  const { mode, stop, samples, fps, selected } = store;
  const pipeline = usePipelineStore();
  const [split, setSplit] = useState(0.5);
  const [turntable, setTurntable] = useState(false);
  const [menu, setMenu] = useState(false);
  const [shots, setShots] = useState<Record<number, string>>({});
  const [lightmaps, setLightmaps] = useState<number | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  const pick = pickSides(view, store.comparison, store.left, store.right);
  const kind = pick?.comparison.kind ?? 'scene';
  const stops = useMemo(() => (kind === 'scene' ? stopsOf(view) : []), [kind, view]);

  const toCompare = useCallback(
    (side: CompareSide | null): CompareModel[] =>
      side
        ? side.members.flatMap(m => {
            const model = view.models.find(x => x.id === m.model);
            const url = model && !model.error ? fileUrl(model.file) : null;
            return url ? [{ id: m.model, url, position: vec3(m.position) }] : [];
          })
        : [],
    [view]
  );
  const left = useMemo(() => toCompare(pick?.left ?? null), [toCompare, pick?.left]);
  const right = useMemo(() => toCompare(pick?.right ?? null), [toCompare, pick?.right]);

  // Start at the first stop, as the site does.
  const started = useRef(false);
  useEffect(() => {
    if (!started.current && stops.length > 0 && !stop) {
      started.current = true;
      store.flyTo(0);
    }
  }, [stops.length, stop, store]);

  useEffect(() => {
    setShots({});
    setReady(false);
    setLoadError(null);
  }, [pick?.comparison.id, pick?.right.id]);

  const rightModels = sideModels(view, pick?.right ?? null);
  const model =
    view.models.find(m => m.id === selected) ??
    rightModels.find(m => m.model === 'character') ??
    rightModels[0] ??
    null;
  const asset = model?.asset ? workspace?.project?.assets?.find(a => a.id === model.asset) : null;
  const thumbPath = asset?.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;

  const savePreset = pipeline.view
    ? (row: CompressPreset) => {
        const rows = pipeline.view?.def.presets ?? [];
        const next = rows.some(r => r.model === row.model)
          ? rows.map(r => (r.model === row.model ? { ...row, lock: r.lock } : r))
          : [...rows, row];
        void pipeline.savePresets(next).then(ok => {
          if (ok) void store.load(view.project);
        });
      }
    : null;

  const menuItems: MenuItem[] = pick
    ? [
        ...view.comparisons.map(c => ({
          id: `c-${c.id}`,
          label: c.label === 'Scene' ? 'Scene' : `${c.label}, versions`,
          icon: (c.kind === 'scene' ? 'squares-four' : 'cube') as 'cube',
          disabled: c.id === pick.comparison.id,
          onSelect: () => store.pick({ comparison: c.id, left: null, right: null }),
        })),
        ...pick.comparison.sides
          .filter(s => s.id !== pick.right.id && s.id !== pick.left?.id)
          .map(s => ({
            id: `l-${s.id}`,
            label: `Compare ${s.label} with ${pick.right.label}`,
            icon: 'arrows-left-right' as const,
            onSelect: () => store.pick({ left: s.id }),
          })),
      ]
    : [];

  const activeStop = stop ? stops[stop.index] : undefined;
  const title = pick
    ? pick.comparison.kind === 'scene'
      ? 'Scene comparison'
      : `${pick.comparison.label} comparison`
    : 'Inspector';
  const totalMb = num(view.scene.bytes) / 1e6;
  const running = Boolean(pipeline.view?.active);

  return (
    <Shell
      right={
        <>
          <SceneBudgetMeter value={totalMb} target={num(view.limits.sceneMb)} />
          <Button
            variant="primary"
            icon="play"
            disabled={running || pipeline.starting || !pipeline.view}
            onClick={() => void pipeline.run({})}
          >
            Run pipeline
          </Button>
        </>
      }
    >
      <main className="flex min-h-0 flex-1">
        <section className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-6 py-[18px]">
          <InspectorTabs />
          <div className="flex items-center gap-3">
            <h1 className="text-panel-title">{title}</h1>
            {pick && menuItems.length > 0 ? (
              <div className="relative">
                <Button size="sm" onClick={() => setMenu(m => !m)} aria-expanded={menu}>
                  {pick.left ? (
                    <>
                      <span className="font-mono">{pick.left.label}</span>
                      <span className="text-fg-mute">vs</span>
                      <span className="font-mono">{pick.right.label}</span>
                    </>
                  ) : (
                    <span className="font-mono">{versusLabel(pick)}</span>
                  )}
                  <Icon name="caret-down" size={12} className="text-fg-mute" />
                </Button>
                {menu ? (
                  <Menu
                    label="Compare"
                    items={menuItems}
                    onClose={() => setMenu(false)}
                    className="absolute top-full left-0 z-20 mt-1"
                  />
                ) : null}
              </div>
            ) : null}
            <div className="ml-auto flex items-center gap-2">
              <SegmentedControl
                label="View mode"
                options={MODES}
                value={mode}
                onChange={m => {
                  setLightmaps(null);
                  store.setMode(m);
                }}
              />
              <IconButton
                icon="arrow-clockwise"
                label={turntable ? 'Stop rotating' : 'Rotate the view'}
                size="sm"
                variant="secondary"
                aria-pressed={turntable}
                onClick={() => setTurntable(t => !t)}
                className={cn(turntable && 'border-clay-line text-clay-text')}
              />
            </div>
          </div>

          {pick ? (
            <div className="relative min-h-[340px] flex-1 overflow-hidden rounded-window border border-line bg-[#141311]">
              <CanvasBoundary resetKey={`${pick.left?.id ?? ''}|${pick.right?.id ?? ''}`}>
                <Suspense fallback={null}>
                  <CompareCanvas
                    className="absolute! inset-0"
                    kind={kind}
                    left={left}
                    right={right}
                    split={pick.left ? split : 0}
                    mode={mode}
                    turntable={turntable}
                    stops={stops}
                    stop={stop}
                    onSample={store.sample}
                    onPick={id => {
                      if (id) store.select(id);
                    }}
                    onStopShots={setShots}
                    onUserMove={store.leaveStop}
                    onLightmaps={setLightmaps}
                    onError={setLoadError}
                    onReady={() => setReady(true)}
                  />
                </Suspense>
              </CanvasBoundary>
              {pick.left ? <SplitHandle split={split} onChange={setSplit} /> : null}
              {pick.left ? (
                <span className={cn(badge, 'top-3 left-3')}>
                  <span className="font-mono">{pick.left.label}</span>
                  {pick.left.note ? <span className="text-fg-dim">{pick.left.note}</span> : null}
                </span>
              ) : null}
              <span className={cn(badge, 'top-3 right-3')}>
                <span className="font-mono">{pick.right.label}</span>
                {pick.right.note ? <span className="text-fg-dim">{pick.right.note}</span> : null}
              </span>
              <span className={cn(badge, 'bottom-3 left-3')}>
                <Icon name="camera" size={12} />
                {activeStop ? `${activeStop.name} · perspective` : 'Free orbit · perspective'}
              </span>
              <span className={cn(badge, 'right-3 bottom-3')}>
                <span
                  className={cn(
                    'size-[7px] rounded-full',
                    fps === 0 ? 'bg-line-strong' : fps >= 55 ? 'bg-sage' : 'bg-amber'
                  )}
                />
                {fps === 0 ? 'WebGL 2 · starting' : `WebGL 2 · ${Math.round(fps)} fps`}
              </span>
              {!ready && !loadError ? (
                <span className="absolute inset-0 grid place-items-center text-fg-mute">
                  Loading {right.length + left.length} models…
                </span>
              ) : null}
              {loadError ? (
                <span className="absolute inset-x-6 top-1/2 -translate-y-1/2 rounded-card bg-danger-soft px-3 py-2 text-center text-danger">
                  {loadError}
                </span>
              ) : null}
              {mode === 'lightmap' && lightmaps === 0 ? (
                <span className={cn(badge, 'top-12 left-1/2 -translate-x-1/2')}>
                  <Icon name="sun" size={12} />
                  No baked lightmap in these models yet. Run Bake light to make one.
                </span>
              ) : null}
            </div>
          ) : (
            <div className="grid min-h-[340px] flex-1 place-items-center rounded-window border border-line">
              <EmptyState
                icon="cube-focus"
                title="Nothing to inspect yet"
                body="Run the pipeline to export the scene, or accept a model from the Inbox. Both show up here to compare."
                action={
                  <Button icon="flow-arrow" onClick={() => setView('pipeline')}>
                    Open the pipeline
                  </Button>
                }
              />
            </div>
          )}

          {stops.length > 0 ? (
            <div className="flex flex-none flex-col gap-2">
              <div className="flex items-baseline justify-between">
                <h2 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
                  Camera stops
                </h2>
                <span className="text-meta text-fg-mute">{stopsHelp(view)}</span>
              </div>
              <div
                className="grid gap-2.5"
                style={{
                  gridTemplateColumns: `repeat(${Math.max(5, stops.length)}, minmax(0, 1fr))`,
                }}
              >
                {stops.map((s, i) => {
                  const src = s.poster ? fileUrl(s.poster) : (shots[i] ?? null);
                  const on = stop?.index === i;
                  return (
                    <button
                      key={`${s.name}-${i}`}
                      type="button"
                      aria-pressed={on}
                      aria-label={`Fly to ${s.name}`}
                      onClick={() => store.flyTo(i)}
                      className={cn(
                        'relative h-[86px] cursor-pointer overflow-hidden rounded-[8px] bg-studio',
                        on
                          ? 'shadow-[0_0_0_2px_var(--color-clay)]'
                          : 'shadow-[0_0_0_1px_var(--color-line)] hover:shadow-[0_0_0_1px_var(--color-line-strong)]'
                      )}
                    >
                      {src ? (
                        <img src={src} alt="" className="block size-full object-cover" />
                      ) : (
                        <span className="grid size-full place-items-center text-fg-mute">
                          <Icon name="camera" />
                        </span>
                      )}
                      <span className={cn(badge, 'bottom-1.5 left-1.5 h-5')}>{s.name}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          ) : null}

          <FrameTime samples={samples} />
        </section>

        <ModelPanel
          view={view}
          model={model}
          thumb={fileUrl(thumbPath)}
          onPickBudget={stem => {
            const found = modelForBudget(view, pick?.right ?? null, stem);
            if (found) store.select(found.id);
          }}
          onPreset={savePreset}
          onExport={m => void store.exportGlb(m.model, m.label)}
          exporting={Boolean(model && store.exporting === model.model)}
        />
      </main>
    </Shell>
  );
}

/** The divider between the two sides, dragged or moved with the arrow keys. */
function SplitHandle({ split, onChange }: { split: number; onChange: (split: number) => void }) {
  const drag = useRef<HTMLDivElement>(null);
  const move = (clientX: number) => {
    const box = drag.current?.parentElement?.getBoundingClientRect();
    if (!box || box.width === 0) return;
    onChange(Math.min(0.98, Math.max(0.02, (clientX - box.left) / box.width)));
  };
  return (
    <div ref={drag} className="pointer-events-none absolute inset-0">
      <span className="absolute inset-y-0 -ml-px w-0.5 bg-fg" style={{ left: `${split * 100}%` }} />
      <button
        type="button"
        role="slider"
        aria-label="Compare position"
        aria-valuenow={Math.round(split * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
        onPointerDown={e => {
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={e => {
          if (e.buttons) move(e.clientX);
        }}
        onKeyDown={e => {
          if (e.key === 'ArrowLeft') onChange(Math.max(0.02, split - 0.05));
          if (e.key === 'ArrowRight') onChange(Math.min(0.98, split + 0.05));
        }}
        className="pointer-events-auto absolute top-1/2 -mt-[18px] -ml-[18px] grid size-9 cursor-ew-resize place-items-center rounded-full bg-fg text-app shadow-[0_4px_16px_rgba(10,9,8,0.45)]"
        style={{ left: `${split * 100}%` }}
      >
        <Icon name="arrows-left-right" />
      </button>
    </div>
  );
}
