import { Dropzone, Icon, cn } from '@maquette/ui';
import { useStageLabels } from '@/state/use-stage-labels';
import { BoardCard } from '../BoardCard';
import { STAGE_ICON } from '../constants';
import { useColumnCards, useColumnDrop } from './BoardColumn.hooks';
import type { BoardColumnProps } from './BoardColumn.types';

/** One stage of the pipeline: its cards, and the drop target for cards from other columns. */
export default function BoardColumn({ stage, assets, boardEmpty, board }: BoardColumnProps) {
  const { over, onDragOver, onDragLeave, onDrop } = useColumnDrop(stage, board.actions.dropOn);
  const { later } = board.actions;
  const STAGE_LABEL = useStageLabels();
  const dropzone = stage === 'concept' && boardEmpty;
  const label = `${STAGE_LABEL[stage]}, ${assets.length} ${assets.length === 1 ? 'asset' : 'assets'}`;
  const { scrollRef, listRef, cards: shown } = useColumnCards(assets.length);
  const cards = assets.slice(shown.start, shown.end).map((asset, n) => (
    <div key={asset.id} {...shown.itemProps(shown.start + n)}>
      <BoardCard asset={asset} board={board} />
    </div>
  ));

  return (
    <section
      ref={scrollRef}
      aria-label={label}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
      className={cn(
        'flex min-h-0 flex-col gap-2 overflow-y-auto rounded-card border border-line bg-panel p-2',
        over && 'border-clay-line bg-clay-soft/40'
      )}
    >
      <div className="flex items-center gap-2 px-1 pt-1 pb-0.5">
        <Icon name={STAGE_ICON[stage]} className="text-fg-mute" />
        <h2 className="text-[13px] leading-5 font-semibold tracking-[-0.01em]">
          {STAGE_LABEL[stage]}
        </h2>
        <span className="font-mono text-[11.5px] text-fg-mute">{assets.length}</span>
      </div>
      {dropzone ? (
        <Dropzone
          title="Drop concept art"
          hint="PNG or JPG, any size"
          icon="image"
          className="h-[176px]"
          onFiles={() => later('Concept import')}
          onActivate={() => later('Concept import')}
        />
      ) : null}
      {assets.length > 0 ? (
        <div
          ref={listRef}
          className="flex flex-none flex-col gap-2"
          style={{ paddingTop: shown.before, paddingBottom: shown.after }}
        >
          {cards}
        </div>
      ) : null}
      {assets.length === 0 && !dropzone ? (
        <div className="grid h-[88px] flex-none place-items-center rounded-card border border-dashed border-line-strong text-meta text-fg-mute">
          Drag an asset here
        </div>
      ) : null}
    </section>
  );
}
