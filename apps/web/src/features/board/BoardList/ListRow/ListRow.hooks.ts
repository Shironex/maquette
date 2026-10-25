import { changedLabel } from '@/state/board-list';
import { KIND_LABEL } from '@/state/board';
import { STAGE_LABEL } from '@/state/pipeline';
import { KIND_ICON } from '@/state/thumbs';
import { assetView } from '../../assetView';
import { STAGE_ICON } from '../../constants';
import type { ListRowProps } from './ListRow.types';

/** What a row shows: the card's picture and send state, and its labels. */
export function useListRow({ row, board, nowMs }: ListRowProps) {
  const { asset } = row;
  const view = assetView(asset, board);
  const checked = board.selection.present.includes(asset.id);
  const status = view.sent?.status ?? row.n.status;
  return {
    ...view,
    status,
    progress: status === 'running' ? view.sent?.progress : undefined,
    checked,
    kindIcon: KIND_ICON[asset.kind],
    kindLabel: KIND_LABEL[asset.kind],
    stageIcon: STAGE_ICON[asset.stage],
    stageLabel: STAGE_LABEL[asset.stage],
    changed: changedLabel(row, nowMs),
    sizeNote: row.sizeSource === 'source' ? 'source' : null,
    menuOpen: board.actions.menuFor === asset.id,
  };
}
