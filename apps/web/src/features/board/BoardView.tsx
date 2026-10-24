import { useEffect, useRef, useState, type DragEvent, type MouseEvent } from 'react';
import {
  AssetCard,
  Button,
  Dropzone,
  Icon,
  SegmentedControl,
  cn,
  type IconName,
} from '@maquette/ui';
import appIcon from '@/assets/app-icon.png';
import { commands, errorMessage, type Asset, type Stage } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { readFlag, writeFlag } from '@/lib/storage';
import { useNativeDrop } from '@/lib/useNativeDrop';
import { useAppStore } from '@/state/app-store';
import { assetCount, cardNumbers, columns, projectFile } from '@/state/board';
import { MAX_BATCH, nextSelection, waitingByHand } from '@/state/batch';
import { useLabStore } from '@/state/lab-store';
import { cardState, isActive, latestJob } from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';
import { TEMPLATE_LABEL } from '@/state/onboarding';
import { STAGES, STAGE_LABEL } from '@/state/pipeline';
import { KIND_ICON } from '@/state/thumbs';
import { Menu } from '@/features/common/Menu';
import { ProviderQueue } from '@/features/providers/ProviderQueue';
import { ProjectTopBar } from '@/features/shell/ProjectTopBar';
import { AddAssetForm } from './AddAssetForm';
import { RenameAssetDialog } from './RenameAssetDialog';

const TIPS_KEY = 'maquette.board.tipsHidden';
const DRAG_TYPE = 'application/x-maquette-asset';
const MODEL_FILE = /\.(glb|zip)$/i;

/** The By hand job of the card under a point, for native GLB drops. */
function jobAt(x: number, y: number): string | null {
  const card = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop-job]');
  return card?.dataset.dropJob ?? null;
}

const STAGE_ICON: Record<Stage, IconName> = {
  concept: 'image',
  refs: 'images',
  tripo: 'cube',
  blender: 'cube-transparent',
  web: 'globe',
  ship: 'rocket-launch',
};

export function BoardView() {
  const workspace = useAppStore(s => s.workspace);
  const setView = useAppStore(s => s.setView);
  const setProject = useAppStore(s => s.setProject);
  const notify = useAppStore(s => s.notify);
  const [tipsHidden, setTipsHidden] = useState(() => readFlag(TIPS_KEY));
  const [adding, setAdding] = useState(false);
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const [renaming, setRenaming] = useState<Asset | null>(null);
  const [dropTarget, setDropTarget] = useState<Stage | null>(null);
  const jobs = useProvidersStore(s => s.jobs);
  const queueOpen = useProvidersStore(s => s.queueOpen);
  const setQueueOpen = useProvidersStore(s => s.setQueueOpen);
  const openPanel = useProvidersStore(s => s.openPanel);
  const showHandoff = useProvidersStore(s => s.showHandoff);
  const handBack = useProvidersStore(s => s.handBack);
  const addModel = useProvidersStore(s => s.addModel);
  const openBatch = useProvidersStore(s => s.openBatch);
  const batchesSent = useProvidersStore(s => s.batchesSent);
  const setChecklistOpen = useProvidersStore(s => s.setChecklistOpen);
  const openObjects = useLabStore(s => s.openObjects);
  const [dropJob, setDropJob] = useState<string | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [anchor, setAnchor] = useState<string | null>(null);
  const project = workspace?.project;
  const assets = project?.assets ?? [];
  const activeJobs = jobs.filter(j => isActive(j.state)).length;
  const byHand = jobs.some(j => j.provider === 'by-hand');
  const toMake = waitingByHand(jobs).length;
  useNativeDrop({
    onOver: ({ x, y }) => setDropJob(jobAt(x, y)),
    onLeave: () => setDropJob(null),
    onDrop: ({ x, y }, paths) => {
      setDropJob(null);
      const job = jobAt(x, y);
      const model = paths.find(p => MODEL_FILE.test(p));
      if (job && model) void handBack(job, model);
      else if (job)
        notify({
          status: 'info',
          title: 'Drop a .glb or .zip file',
          detail: 'The card takes the GLB you made by hand, or the zip the web app downloads.',
        });
    },
  });
  const budget = workspace?.config.budgets.sceneMb ?? 3;
  const byStage = columns(assets);
  const order = STAGES.flatMap(stage => byStage[stage].map(a => a.id));
  const present = selected.filter(id => order.includes(id));

  // A selection, an open menu or dialog belongs to the project it was made in.
  useEffect(() => {
    setSelected([]);
    setAnchor(null);
    setRenaming(null);
    setAdding(false);
    setMenuFor(null);
  }, [project?.id]);
  // A confirmed Send to 3D hands the selection over, so the cards let go.
  const lastSent = useRef(batchesSent);
  useEffect(() => {
    if (lastSent.current === batchesSent) return;
    lastSent.current = batchesSent;
    setSelected([]);
    setAnchor(null);
  }, [batchesSent]);
  useEffect(() => {
    if (selected.length === 0) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !document.querySelector('[aria-modal="true"]')) setSelected([]);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selected.length]);

  const check = (id: string, on: boolean, range: boolean) => {
    setSelected(s => nextSelection(s, order, id, on, range, anchor));
    setAnchor(id);
  };
  /** Shift and a click anywhere on a card selects it, and the cards between. */
  const shiftClick = (id: string) => (e: MouseEvent) => {
    if (!e.shiftKey) return;
    const target = e.target as HTMLElement;
    if (target.closest('[aria-haspopup], [role="menu"], [role="checkbox"], [data-card-action]'))
      return;
    e.preventDefault();
    check(id, !present.includes(id), true);
  };
  const sendSelected = () => {
    if (!project || present.length === 0) return;
    void openBatch(project.id, present.slice(0, MAX_BATCH));
  };

  const hideTips = () => {
    setTipsHidden(true);
    writeFlag(TIPS_KEY, true);
  };
  const later = (what: string) =>
    notify({
      status: 'info',
      title: `${what} arrives in a later version`,
      detail: 'This build runs the pipeline and checks models in the inbox.',
    });

  const move = async (asset: Asset, stage: Stage) => {
    if (!project || asset.stage === stage) return;
    try {
      setProject(await commands.assetMove(project.id, asset.id, stage));
    } catch (error) {
      notify({
        status: 'failed',
        title: `Could not move ${asset.name}`,
        detail: errorMessage(error),
      });
    }
  };
  const openInBlender = async (asset: Asset) => {
    if (!project) return;
    try {
      await commands.assetOpenInBlender(project.id, asset.id);
    } catch (error) {
      notify({ status: 'failed', title: 'Could not open Blender', detail: errorMessage(error) });
    }
  };
  const onDrop = (stage: Stage) => (e: DragEvent) => {
    e.preventDefault();
    setDropTarget(null);
    const id = e.dataTransfer.getData(DRAG_TYPE);
    const asset = assets.find(a => a.id === id);
    if (asset) void move(asset, stage);
  };

  return (
    <>
      <ProjectTopBar />
      <main className="flex min-h-0 flex-1 flex-col gap-[18px] px-6 pt-[22px] pb-5">
        <div className="flex items-end gap-4">
          <div className="flex flex-col gap-1">
            <h1 className="text-screen-title">{project?.name ?? 'No project'}</h1>
            <p className="text-fg-mute">
              {project
                ? `${assetCount(assets.length)} · ${TEMPLATE_LABEL[project.template]} · created ${formatAgo(Date.parse(project.createdAt), Date.now())}`
                : 'Create a project in Settings to start.'}
            </p>
          </div>
          <div className="relative ml-auto flex items-center gap-2">
            <SegmentedControl
              label="Board layout"
              options={[
                { value: 'board', label: 'Board', icon: 'squares-four' },
                { value: 'list', label: 'List', icon: 'list' },
              ]}
              defaultValue="board"
            />
            {byHand ? (
              <Button icon="hand-pointing" onClick={() => setChecklistOpen(true)}>
                By hand
                {toMake > 0 ? (
                  <span className="ml-0.5 rounded-pill bg-clay-soft px-1.5 font-mono text-[11px] leading-[18px] text-clay-text">
                    {toMake}
                  </span>
                ) : null}
              </Button>
            ) : null}
            <div className="relative">
              <Button
                icon="cube"
                data-queue-toggle
                aria-expanded={queueOpen}
                disabled={!project}
                onClick={() => setQueueOpen(!queueOpen)}
              >
                3D queue
                {activeJobs > 0 ? (
                  <span className="ml-0.5 rounded-pill bg-amber-soft px-1.5 font-mono text-[11px] leading-[18px] text-amber">
                    {activeJobs}
                  </span>
                ) : null}
              </Button>
              {queueOpen ? <ProviderQueue onClose={() => setQueueOpen(false)} /> : null}
            </div>
            <Button
              icon="plus"
              disabled={!project}
              aria-expanded={adding}
              onClick={() => setAdding(v => !v)}
            >
              Add asset
            </Button>
            {adding && project ? (
              <AddAssetForm
                project={project.id}
                onDone={next => {
                  setProject(next);
                  setAdding(false);
                }}
                onCancel={() => setAdding(false)}
              />
            ) : null}
          </div>
        </div>

        <div className="relative min-h-0 flex-1">
          <div className="absolute inset-0 grid grid-cols-6 gap-3">
            {STAGES.map(stage => {
              const list = byStage[stage];
              return (
                <section
                  key={stage}
                  aria-label={`${STAGE_LABEL[stage]}, ${list.length} ${list.length === 1 ? 'asset' : 'assets'}`}
                  onDragOver={e => {
                    if (e.dataTransfer.types.includes(DRAG_TYPE)) {
                      e.preventDefault();
                      setDropTarget(stage);
                    }
                  }}
                  onDragLeave={() => setDropTarget(t => (t === stage ? null : t))}
                  onDrop={onDrop(stage)}
                  className={cn(
                    'flex min-h-0 flex-col gap-2 overflow-y-auto rounded-card border border-line bg-panel p-2',
                    dropTarget === stage && 'border-clay-line bg-clay-soft/40'
                  )}
                >
                  <div className="flex items-center gap-2 px-1 pt-1 pb-0.5">
                    <Icon name={STAGE_ICON[stage]} className="text-fg-mute" />
                    <h2 className="text-[13px] leading-5 font-semibold tracking-[-0.01em]">
                      {STAGE_LABEL[stage]}
                    </h2>
                    <span className="font-mono text-[11.5px] text-fg-mute">{list.length}</span>
                  </div>
                  {stage === 'concept' && assets.length === 0 ? (
                    <Dropzone
                      title="Drop concept art"
                      hint="PNG or JPG, any size"
                      icon="image"
                      className="h-[176px]"
                      onFiles={() => later('Concept import')}
                      onActivate={() => later('Concept import')}
                    />
                  ) : null}
                  {list.map(asset => {
                    const n = cardNumbers(asset);
                    const thumbPath =
                      asset.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;
                    const others = STAGES.filter(s => s !== asset.stage);
                    const job = latestJob(jobs, asset.id);
                    const sent = cardState(job);
                    const waiting = job?.state === 'waiting' ? job : undefined;
                    const refPicture = asset.thumbnail?.startsWith('refs/')
                      ? asset.thumbnail
                      : null;
                    return (
                      <AssetCard
                        key={asset.id}
                        data-drop-job={waiting?.id}
                        checked={present.includes(asset.id)}
                        selecting={present.length > 0}
                        onCheck={(on, range) => check(asset.id, on, range)}
                        onClickCapture={shiftClick(asset.id)}
                        actions={
                          waiting ? (
                            <Button
                              size="sm"
                              icon="upload-simple"
                              data-card-action
                              onClick={() => void addModel(waiting.id)}
                            >
                              Add model...
                            </Button>
                          ) : undefined
                        }
                        className={cn(
                          waiting && 'border-dashed border-clay-line',
                          dropJob &&
                            dropJob === waiting?.id &&
                            'border-solid border-clay bg-clay-soft/40'
                        )}
                        name={asset.name}
                        thumb={fileUrl(thumbPath) ?? undefined}
                        placeholder={{
                          icon: KIND_ICON[asset.kind],
                          label: asset.versions.length > 0 ? 'No preview yet' : undefined,
                        }}
                        meta={asset.versions.length === 0 ? 'No model yet' : undefined}
                        version={n.version}
                        latest={n.version !== undefined}
                        tris={n.tris}
                        size={n.size}
                        status={sent?.status ?? n.status}
                        statusLabel={sent?.label}
                        statusIcon={sent?.icon}
                        progress={sent?.progress}
                        progressLabel={sent?.progressLabel}
                        draggable
                        onDragStart={e => {
                          e.dataTransfer.setData(DRAG_TYPE, asset.id);
                          e.dataTransfer.effectAllowed = 'move';
                        }}
                        onMore={() => setMenuFor(m => (m === asset.id ? null : asset.id))}
                        moreOpen={menuFor === asset.id}
                        menu={
                          <Menu
                            label={`${asset.name} actions`}
                            onClose={() => setMenuFor(null)}
                            items={[
                              ...(waiting
                                ? [
                                    {
                                      id: 'handoff',
                                      label: 'Settings card',
                                      icon: 'file-text' as const,
                                      onSelect: () => showHandoff(waiting),
                                    },
                                    {
                                      id: 'hand-back',
                                      label: 'Add model...',
                                      icon: 'upload-simple' as const,
                                      onSelect: () => void addModel(waiting.id),
                                    },
                                  ]
                                : []),
                              {
                                id: 'send',
                                label: 'Send to 3D',
                                icon: 'cube',
                                onSelect: () => void openPanel(project?.id ?? '', asset.id),
                              },
                              ...(refPicture && workspace && project
                                ? [
                                    {
                                      id: 'objects',
                                      label: 'Make object refs...',
                                      icon: 'cube-focus' as const,
                                      onSelect: () =>
                                        openObjects({
                                          project: project.id,
                                          source: refPicture,
                                          sourcePath: projectFile(workspace, refPicture) ?? '',
                                          sourceName: asset.name,
                                        }),
                                    },
                                  ]
                                : []),
                              {
                                id: 'rename',
                                label: 'Rename...',
                                icon: 'file-text',
                                onSelect: () => setRenaming(asset),
                              },
                              ...others.map(s => ({
                                id: `move-${s}`,
                                label: `Move to ${STAGE_LABEL[s]}`,
                                icon: STAGE_ICON[s],
                                onSelect: () => void move(asset, s),
                              })),
                              {
                                id: 'blender',
                                label: 'Open in Blender',
                                icon: 'arrow-square-out',
                                disabled: asset.versions.length === 0,
                                onSelect: () => void openInBlender(asset),
                              },
                            ]}
                          />
                        }
                      />
                    );
                  })}
                  {list.length === 0 && !(stage === 'concept' && assets.length === 0) ? (
                    <div className="grid h-[88px] flex-none place-items-center rounded-card border border-dashed border-line-strong text-meta text-fg-mute">
                      Drag an asset here
                    </div>
                  ) : null}
                </section>
              );
            })}
          </div>

          {renaming && project ? (
            <RenameAssetDialog
              key={renaming.id}
              project={project.id}
              asset={renaming}
              onDone={next => {
                setProject(next);
                setRenaming(null);
              }}
              onClose={() => setRenaming(null)}
            />
          ) : null}

          {present.length > 0 ? (
            <SelectionBar
              count={present.length}
              onSend={sendSelected}
              onClear={() => setSelected([])}
            />
          ) : null}

          {tipsHidden || !project || assets.length > 0 ? null : (
            <FirstRunCard
              name={project.name}
              budget={budget}
              onLab={() => setView('lab')}
              onImport={() => setView('inbox')}
              onHide={hideTips}
            />
          )}
        </div>
      </main>
    </>
  );
}

function SelectionBar({
  count,
  onSend,
  onClear,
}: {
  count: number;
  onSend: () => void;
  onClear: () => void;
}) {
  const over = count > MAX_BATCH;
  return (
    <div
      role="toolbar"
      aria-label="Selected assets"
      className="absolute bottom-4 left-1/2 z-20 flex -translate-x-1/2 items-center gap-3 rounded-card border border-line-strong bg-panel py-2 pr-2 pl-3.5 shadow-window"
    >
      <span className="grid size-[22px] place-items-center rounded-[6px] bg-clay text-clay-ink">
        <Icon name="check" size={13} />
      </span>
      <span className="font-semibold whitespace-nowrap">
        {count} {count === 1 ? 'asset' : 'assets'} selected
      </span>
      <span className="text-meta whitespace-nowrap text-fg-mute">
        {over ? `The first ${MAX_BATCH} go in one batch` : 'Shift-click to add a range'}
      </span>
      <Button variant="primary" icon="cube" onClick={onSend}>
        Send to 3D
      </Button>
      <Button variant="ghost" onClick={onClear}>
        Clear
      </Button>
    </div>
  );
}

const STEPS = [
  {
    title: 'Drop a concept',
    detail: 'Drag concept art onto the Concept column, or generate a few directions in the Lab.',
  },
  {
    title: 'Send picks to 3D',
    detail:
      'Pick the concepts you like and send them to Tripo, Meshy or by hand. I check every model that comes back.',
  },
  {
    title: 'Run the pipeline',
    detail:
      'Blender builds, bakes and exports the scene, then Compress fits it into the {budget} MB budget.',
  },
];

function FirstRunCard({
  name,
  budget,
  onLab,
  onImport,
  onHide,
}: {
  name: string;
  budget: number;
  onLab: () => void;
  onImport: () => void;
  onHide: () => void;
}) {
  return (
    <section
      aria-labelledby="first-run"
      className="absolute top-1/2 left-1/2 w-[560px] max-w-[calc(100%-32px)] -translate-x-1/2 -translate-y-[46%] rounded-[14px] border border-line-strong bg-panel px-7 pt-7 pb-6 shadow-window"
    >
      <div className="flex items-center gap-3.5">
        <img src={appIcon} alt="" className="size-12 rounded-[12px]" />
        <div>
          <h2 id="first-run" className="text-panel-title">
            Your first build in three steps
          </h2>
          <p className="mt-0.5 text-fg-dim">
            {name} is empty. Start with a concept and I will carry it through.
          </p>
        </div>
      </div>
      <ol className="mt-5 list-none p-0">
        {STEPS.map((step, i) => (
          <li key={step.title} className="flex gap-3.5 border-t border-line py-3">
            <span className="grid size-7 flex-none place-items-center rounded-full border-[1.5px] border-clay-line font-mono text-[12px] leading-none font-semibold text-clay-text">
              {i + 1}
            </span>
            <div>
              <div className="text-[14px] font-semibold">{step.title}</div>
              <div className="mt-[3px] text-[12.5px] leading-[18px] text-fg-mute">
                {step.detail.replace('{budget}', budget.toFixed(1))}
              </div>
            </div>
          </li>
        ))}
      </ol>
      <div className="mt-[18px] flex items-center gap-2.5 border-t border-line pt-[18px]">
        <Button size="lg" variant="primary" icon="flask" onClick={onLab}>
          Open Concept lab
        </Button>
        <Button size="lg" icon="download-simple" onClick={onImport}>
          Import models
        </Button>
        <Button variant="ghost" className="ml-auto" onClick={onHide}>
          Hide tips
        </Button>
      </div>
    </section>
  );
}
