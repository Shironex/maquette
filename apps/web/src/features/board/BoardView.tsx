import { useState } from 'react';
import {
  BudgetMeter,
  Button,
  Dropzone,
  Icon,
  SegmentedControl,
  StageStepper,
  TopBar,
  PIPELINE_STAGES,
  type IconName,
} from '@maquette/ui';
import appIcon from '@/assets/app-icon.png';
import { formatAgo } from '@/lib/format';
import { readFlag, writeFlag } from '@/lib/storage';
import { useAppStore } from '@/state/app-store';
import { TEMPLATE_LABEL } from '@/state/onboarding';

const TIPS_KEY = 'maquette.board.tipsHidden';

const COLUMNS: readonly { label: string; icon: IconName; slots: number[] }[] = [
  { label: 'Concept', icon: 'image', slots: [88] },
  { label: 'Refs', icon: 'images', slots: [88, 88] },
  { label: 'Tripo', icon: 'cube', slots: [88, 88] },
  { label: 'Blender', icon: 'cube-transparent', slots: [88, 88, 88] },
  { label: 'Web', icon: 'globe', slots: [88, 88] },
  { label: 'Ship', icon: 'rocket-launch', slots: [176] },
];

const STAGES = PIPELINE_STAGES.map((label, i) => ({
  id: label.toLowerCase(),
  label,
  state: i === 0 ? ('current' as const) : ('todo' as const),
}));

export function BoardView() {
  const workspace = useAppStore(s => s.workspace);
  const setView = useAppStore(s => s.setView);
  const notify = useAppStore(s => s.notify);
  const [tipsHidden, setTipsHidden] = useState(() => readFlag(TIPS_KEY));
  const project = workspace?.project;
  const budget = workspace?.config.budgets.sceneMb ?? 3;

  const hideTips = () => {
    setTipsHidden(true);
    writeFlag(TIPS_KEY, true);
  };
  const later = (what: string) =>
    notify({
      status: 'info',
      title: `${what} arrives in a later version`,
      detail: 'This build sets up the workspace and the toolchain.',
    });

  return (
    <>
      <TopBar
        lead={<StageStepper stages={STAGES} />}
        right={
          <>
            <BudgetMeter label="Scene budget" value={0} target={budget} unit="MB" />
            <Button icon="play" disabled title="Add assets first">
              Run pipeline
            </Button>
          </>
        }
      />
      <main className="flex min-h-0 flex-1 flex-col gap-[18px] px-6 pt-[22px] pb-5">
        <div className="flex items-end gap-4">
          <div className="flex flex-col gap-1">
            <h1 className="text-screen-title">{project?.name ?? 'No project'}</h1>
            <p className="text-fg-mute">
              {project
                ? `No assets yet · ${TEMPLATE_LABEL[project.template]} · created ${formatAgo(Date.parse(project.createdAt), Date.now())}`
                : 'Create a project in Settings to start.'}
            </p>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <SegmentedControl
              label="Board layout"
              options={[
                { value: 'board', label: 'Board', icon: 'squares-four' },
                { value: 'list', label: 'List', icon: 'list' },
              ]}
              defaultValue="board"
            />
            <Button icon="plus" onClick={() => later('Adding assets')}>
              Add asset
            </Button>
          </div>
        </div>

        <div className="relative min-h-0 flex-1">
          <div className="absolute inset-0 grid grid-cols-6 gap-3">
            {COLUMNS.map(col => (
              <section
                key={col.label}
                aria-label={`${col.label}, empty`}
                className="flex flex-col gap-2 rounded-card border border-line bg-panel p-2"
              >
                <div className="flex items-center gap-2 px-1 pt-1 pb-0.5">
                  <Icon name={col.icon} className="text-fg-mute" />
                  <h2 className="text-[13px] leading-5 font-semibold tracking-[-0.01em]">
                    {col.label}
                  </h2>
                  <span className="font-mono text-[11.5px] text-fg-mute">0</span>
                </div>
                {col.label === 'Concept' ? (
                  <Dropzone
                    title="Drop concept art"
                    hint="PNG or JPG, any size"
                    icon="image"
                    className="h-[176px]"
                    onFiles={() => later('Concept import')}
                    onActivate={() => later('Concept import')}
                  />
                ) : null}
                {col.slots.map((h, i) => (
                  <div
                    key={i}
                    aria-hidden="true"
                    className="rounded-card border border-dashed border-line-strong"
                    style={{ height: h }}
                  />
                ))}
              </section>
            ))}
          </div>

          {tipsHidden || !project ? null : (
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
