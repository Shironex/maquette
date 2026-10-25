import { ProjectTopBar } from '@/features/common/ProjectTopBar';
import { STAGES } from '@/state/pipeline';
import type { BoardEnv } from '../board.types';
import { BoardColumn } from '../BoardColumn';
import { BoardHeader } from '../BoardHeader';
import { BoardList } from '../BoardList';
import { FirstRunCard } from '../FirstRunCard';
import { PrepDialog } from '../PrepDialog';
import { RenameAssetDialog } from '../RenameAssetDialog';
import { SelectionBar } from '../SelectionBar';
import { useBoardData, useBoardSelection, useCardActions, useCardDrop } from './BoardView.hooks';
import type { BoardViewProps } from './BoardView.types';

/** The project's assets in one column per pipeline stage. */
export default function BoardView({ queue }: BoardViewProps) {
  const { workspace, project, assets, jobs, otherProjects, prefs, byStage, rows, order, budget } =
    useBoardData();
  const selection = useBoardSelection(project?.id, order);
  const actions = useCardActions(project);
  const dropJob = useCardDrop();
  const board: BoardEnv = { workspace, project, jobs, otherProjects, dropJob, selection, actions };
  const { present } = selection;
  const { renaming, sizing } = actions;
  const stageColumns = STAGES.map(stage => (
    <BoardColumn
      key={stage}
      stage={stage}
      assets={byStage[stage]}
      boardEmpty={assets.length === 0}
      board={board}
    />
  ));

  return (
    <>
      <ProjectTopBar />
      <main className="flex min-h-0 flex-1 flex-col gap-[18px] px-6 pt-[22px] pb-5">
        <BoardHeader queue={queue} />

        <div className="relative min-h-0 flex-1">
          {prefs.layout === 'list' && assets.length > 0 ? (
            <BoardList rows={rows} total={assets.length} prefs={prefs} board={board} />
          ) : (
            <div className="absolute inset-0 grid grid-cols-6 gap-3">{stageColumns}</div>
          )}

          {renaming && project ? (
            <RenameAssetDialog
              key={renaming.id}
              project={project.id}
              asset={renaming}
              onDone={actions.renamed}
              onClose={() => actions.setRenaming(null)}
            />
          ) : null}

          {sizing && project ? (
            <PrepDialog
              key={sizing.id}
              project={project.id}
              asset={sizing}
              onClose={() => actions.setSizing(null)}
            />
          ) : null}

          {present.length > 0 ? (
            <SelectionBar
              count={present.length}
              onSend={selection.send}
              onClear={selection.clear}
            />
          ) : null}

          {!project || assets.length > 0 ? null : (
            <FirstRunCard name={project.name} budget={budget} />
          )}
        </div>
      </main>
    </>
  );
}
