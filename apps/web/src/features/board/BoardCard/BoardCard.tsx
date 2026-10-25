import { AssetCard, Button, cn } from '@maquette/ui';
import { KIND_ICON } from '@/state/thumbs';
import { useBoardCard } from './BoardCard.hooks';
import type { BoardCardProps } from './BoardCard.types';
import { CardMenu } from './CardMenu';

/** One asset on the Board: its picture, numbers, latest send and menu. */
export default function BoardCard({ asset, board }: BoardCardProps) {
  const { n, thumb, sent, waiting, refPicture, addModel, onDragStart } = useBoardCard(asset, board);
  const { selection, actions, dropJob } = board;
  const { present } = selection;

  return (
    <AssetCard
      data-drop-job={waiting?.id}
      checked={present.includes(asset.id)}
      selecting={present.length > 0}
      onCheck={(on, range) => selection.check(asset.id, on, range)}
      onClickCapture={selection.shiftClick(asset.id)}
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
        dropJob && dropJob === waiting?.id && 'border-solid border-clay bg-clay-soft/40'
      )}
      name={asset.name}
      thumb={thumb}
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
      onDragStart={onDragStart}
      onMore={() => {
        actions.setCopyFor(null);
        actions.setMenuFor(m => (m === asset.id ? null : asset.id));
      }}
      moreOpen={actions.menuFor === asset.id}
      menu={<CardMenu asset={asset} waiting={waiting} refPicture={refPicture} board={board} />}
    />
  );
}
