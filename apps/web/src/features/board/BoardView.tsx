import { useState, type DragEvent } from 'react';
import {
  AssetCard,
  BudgetMeter,
  Button,
  Dropzone,
  Icon,
  SegmentedControl,
  StageStepper,
  TopBar,
  cn,
  type IconName,
} from '@maquette/ui';
import appIcon from '@/assets/app-icon.png';
import { commands, errorMessage, type Asset, type Stage } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { readFlag, writeFlag } from '@/lib/storage';
import { useAppStore } from '@/state/app-store';
import { assetCount, cardNumbers, columns, projectFile } from '@/state/board';
import { TEMPLATE_LABEL } from '@/state/onboarding';
import { STAGES, STAGE_LABEL, estimatedTransfer, stageViews } from '@/state/pipeline';
import { usePipelineStore } from '@/state/pipeline-store';
import { KIND_ICON } from '@/state/thumbs';
import { Menu } from '@/features/common/Menu';
import { AddAssetForm } from './AddAssetForm';

const TIPS_KEY = 'maquette.board.tipsHidden';
const DRAG_TYPE = 'application/x-maquette-asset';

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
  const pipeline = usePipelineStore(s => s.view);
  const runPipeline = usePipelineStore(s => s.run);
  const [tipsHidden, setTipsHidden] = useState(() => readFlag(TIPS_KEY));
  const [adding, setAdding] = useState(false);
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<Stage | null>(null);
  const project = workspace?.project;
  const assets = project?.assets ?? [];
  const budget = workspace?.config.budgets.sceneMb ?? 3;
  const latest = pipeline?.runs[0];
  const running = Boolean(pipeline?.active);
  const byStage = columns(assets);

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
  const run = () => {
    void runPipeline();
    setView('pipeline');
  };

  return (
    <>
      <TopBar
        lead={<StageStepper stages={stageViews(assets, latest)} />}
        right={
          <>
            <BudgetMeter
              label="Scene budget"
              value={pipeline ? estimatedTransfer(pipeline) / 1e6 : 0}
              target={budget}
              unit="MB"
            />
            {running ? (
              <Button icon="flow-arrow" onClick={() => setView('pipeline')}>
                Run in progress
              </Button>
            ) : (
              <Button
                icon="play"
                disabled={!pipeline || pipeline.def.steps.length === 0}
                title={pipeline ? 'Runs every step, skipping cached ones' : 'Loading the pipeline'}
                onClick={run}
              >
                Run pipeline
              </Button>
            )}
          </>
        }
      />
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
                    return (
                      <AssetCard
                        key={asset.id}
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
                        status={n.status}
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

const STEPS = [
  {
    title: 'Drop a concept',
    detail: 'Drag concept art onto the Concept column, or generate a few directions in the Lab.',
  },
  {
    title: 'Send picks to Tripo',
    detail:
      'Star the concepts you like. I export refs with your Tripo preset and check every model that comes back.',
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
