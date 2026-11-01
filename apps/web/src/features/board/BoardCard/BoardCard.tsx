import { AssetCard, Button, cn } from '@maquette/ui';
import { builtChip, builtMeta, wantsRebuild } from '@/state/built';
import { KIND_ICON } from '@/state/thumbs';
import { useBoardCard } from './BoardCard.hooks';
import { viewsLine } from '@/state/refs';
import type { BoardCardProps } from './BoardCard.types';
import { CardMenu } from './CardMenu';
import { ExportName } from './ExportName';

/** One asset on the Board: its picture, numbers, latest send or build, and menu. */
export default function BoardCard({ asset, board }: BoardCardProps) {
  const card = useBoardCard(asset, board);
  const { n, over, thumb, sent, waiting, refPicture, addModel, onDragStart, built, rebuild } = card;
  const update = card.libraryUpdate;
  // A chip word can run past the card: it ends in an ellipsis and its tooltip has it whole.
  const warning = !built && over && !sent ? over : undefined;
  const chip = built
    ? builtChip(built.state)
    : warning
      ? { state: 'review' as const, label: warning, icon: 'warning' as const }
      : undefined;
  const { selection, actions, dropJob } = board;
  const { present } = selection;

  return (
    <AssetCard
      data-drop-job={waiting?.id}
      selection={{
        checked: present.includes(asset.id),
        selecting: present.length > 0,
        onCheck: (on, range) => selection.check(asset.id, on, range),
      }}
      onClickCapture={selection.shiftClick(asset.id)}
      onContextMenu={card.onContextMenu}
      actions={
        waiting ? (
          <Button
            size="sm"
            icon="upload-simple"
            tooltip="Choose the GLB, or the .zip the site downloads, that you made for this card"
            data-card-action
            className="max-w-full"
            onClick={() => void addModel(waiting.id)}
          >
            <span className="min-w-0 truncate @max-[120px]:sr-only">Add model...</span>
          </Button>
        ) : built && wantsRebuild(built.state) ? (
          <Button
            size="sm"
            icon="arrow-clockwise"
            tooltip={`Runs ${built.source.script} in Blender headless again`}
            data-card-action
            className="max-w-full"
            onClick={rebuild}
          >
            <span className="min-w-0 truncate @max-[120px]:sr-only">Rebuild</span>
          </Button>
        ) : update ? (
          <Button
            size="sm"
            icon="arrow-up"
            data-card-action
            className="max-w-full"
            tooltip={`Update from the library: it has v${update.to} of ${update.entry.entry.name}, this card is on v${update.from}`}
            onClick={card.openUpdate}
          >
            <span className="min-w-0 truncate @max-[120px]:sr-only">Library has v{update.to}</span>
          </Button>
        ) : undefined
      }
      className={cn(
        waiting && 'border-dashed border-clay-line',
        dropJob && dropJob === waiting?.id && 'border-solid border-clay bg-clay-soft/40'
      )}
      name={asset.name}
      picture={{
        thumb,
        placeholder: {
          icon: KIND_ICON[asset.kind],
          label: card.previewing
            ? 'Rendering a preview'
            : asset.versions.length > 0
              ? 'No preview yet'
              : undefined,
          busy: card.previewing,
        },
      }}
      details={{
        meta:
          asset.versions.length > 0
            ? undefined
            : built
              ? builtMeta(built)
              : (asset.refs ?? []).length > 0
                ? `No model yet · ${viewsLine(asset.refs ?? [])}`
                : 'No model yet',
        tris: n.tris,
        size: n.size,
      }}
      version={n.version}
      latest={n.version !== undefined}
      status={{
        state: chip?.state ?? sent?.status ?? n.status,
        label: chip?.label ?? sent?.label,
        icon: chip?.icon ?? sent?.icon,
        progress: sent?.progress,
        progressLabel: sent?.progressLabel,
        tooltip: warning ?? sent?.label,
        note: card.exportName ? <ExportName name={card.exportName} /> : undefined,
      }}
      draggable
      onDragStart={onDragStart}
      more={{
        onOpen: () => {
          actions.setCopyFor(null);
          actions.setMenuFor(m => (m === asset.id ? null : asset.id));
        },
        open: actions.menuFor === asset.id,
        menu: (
          <CardMenu
            asset={asset}
            waiting={waiting}
            refPicture={refPicture}
            built={built}
            board={board}
          />
        ),
      }}
    />
  );
}
