import { Button, SegmentedControl } from '@maquette/ui';
import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import { assetCount } from '@/state/board';
import { TEMPLATE_LABEL } from '@/state/onboarding';
import { AddAssetForm } from '../AddAssetForm';
import { RecentlyRemoved } from '../RecentlyRemoved';
import { useBoardHeader } from './BoardHeader.hooks';
import type { BoardHeaderProps } from './BoardHeader.types';

/** The project name and what it holds, then the Board's buttons. */
export default function BoardHeader({ queue }: BoardHeaderProps) {
  const {
    project,
    layout,
    setLayout,
    adding,
    setAdding,
    added,
    activeJobs,
    byHand,
    toMake,
    queueOpen,
    setQueueOpen,
    setChecklistOpen,
    addBuilt,
    game,
    importModels,
    missing,
    rendering,
    renderPreviews,
  } = useBoardHeader();
  const subtitle = project
    ? `${assetCount((project.assets ?? []).length)} · ${game ? `game · .${game.format} in ${game.modelsDir}/` : TEMPLATE_LABEL[project.template]} · created ${formatAgo(Date.parse(project.createdAt), clock.nowMs())}`
    : 'Create a project in Settings to start.';

  return (
    <div className="flex items-end gap-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-screen-title">{project?.name ?? 'No project'}</h1>
        <p className="text-fg-mute">{subtitle}</p>
      </div>
      <div className="relative ml-auto flex items-center gap-2">
        <SegmentedControl
          label="Board layout"
          options={[
            { value: 'board', label: 'Board', icon: 'squares-four', hint: 'One column per stage' },
            { value: 'list', label: 'List', icon: 'list', hint: 'One sortable row per asset' },
          ]}
          value={layout}
          onChange={setLayout}
        />
        {byHand ? (
          <Button
            icon="hand-pointing"
            tooltip="By hand: models waiting for you to make on the provider's site"
            onClick={() => setChecklistOpen(true)}
          >
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
            disabledReason="Open a project first"
            tooltip="3D queue: sends to Tripo or Meshy and their progress"
            onClick={() => setQueueOpen(!queueOpen)}
          >
            3D queue
            {activeJobs > 0 ? (
              <span className="ml-0.5 rounded-pill bg-amber-soft px-1.5 font-mono text-[11px] leading-[18px] text-amber">
                {activeJobs}
              </span>
            ) : null}
          </Button>
          {queueOpen ? queue(() => setQueueOpen(false)) : null}
        </div>
        <RecentlyRemoved />
        {missing > 0 ? (
          <Button
            icon="image"
            loading={rendering}
            tooltip="Renders each model without a preview, headless in Blender, one at a time"
            onClick={renderPreviews}
          >
            {missing === 1 ? 'Render 1 preview' : `Render ${missing} previews`}
          </Button>
        ) : null}
        {game ? (
          <Button
            icon="download-simple"
            tooltip="Import models: bring existing game models into this project"
            onClick={importModels}
          >
            Import models
          </Button>
        ) : null}
        <Button
          icon="wrench"
          disabled={!project}
          disabledReason="Open a project first"
          tooltip="Built asset: a model a Blender script in the project makes"
          onClick={addBuilt}
        >
          Built asset
        </Button>
        <Button
          icon="plus"
          disabled={!project}
          disabledReason="Open a project first"
          tooltip="Add asset: a new card on the Board, named, with its kind and column"
          aria-expanded={adding}
          onClick={() => setAdding(v => !v)}
        >
          Add asset
        </Button>
        {adding && project ? (
          <AddAssetForm project={project.id} onDone={added} onCancel={() => setAdding(false)} />
        ) : null}
      </div>
    </div>
  );
}
