import { changedLabel } from '@/state/board-list';
import { KIND_LABEL } from '@/state/board';
import { useStageLabels } from '@/state/use-stage-labels';
import { KIND_ICON } from '@/state/thumbs';
import { useBudgetCheck } from '@/state/use-budget-check';
import { useRendersPreview } from '@/state/use-game-store';
import { trianglesLine } from '@/state/game';
import { assetView } from '../../assetView';
import { STAGE_ICON } from '../../constants';
import type { ListRowProps } from './ListRow.types';

/** What a row shows: the card's picture and send state, and its labels. */
export function useListRow({ row, board, nowMs }: ListRowProps) {
  const { asset } = row;
  const labels = useStageLabels();
  const budget = useBudgetCheck(asset);
  const view = assetView(asset, board);
  const previewing = useRendersPreview(asset, board.project?.id);
  const checked = board.selection.present.includes(asset.id);
  const status = view.sent?.status ?? row.n.status;
  return {
    ...view,
    previewing,
    status,
    progress: status === 'running' ? view.sent?.progress : undefined,
    checked,
    kindIcon: KIND_ICON[asset.kind],
    kindLabel: KIND_LABEL[asset.kind],
    stageIcon: STAGE_ICON[asset.stage],
    stageLabel: labels[asset.stage],
    budgetLine: budget ? trianglesLine(budget) : null,
    over: budget?.problems.length ? budget.problems.join(', ') : null,
    changed: changedLabel(row, nowMs),
    sizeNote: row.sizeSource === 'source' ? 'source' : null,
    menuOpen: board.actions.menuFor === asset.id,
  };
}
