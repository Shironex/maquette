import { Icon, cn } from '@maquette/ui';
import type { AnimationItem } from '@/ipc/client';
import { RIG_LABEL, lengthLine } from '@/state/animations';
import { CARD, CARD_BUTTON } from '@/features/common/cardButton';
import { ClipPose } from '../ClipPose';

export interface ClipCardProps {
  item: AnimationItem;
  selected: boolean;
  onSelect: () => void;
}

const MARK =
  'inline-flex items-center gap-1 rounded-pill border border-line px-1.5 font-mono text-[11px] leading-[18px] text-fg-mute';

/** Whether the clip travels or stays put, and whether it loops. */
export function ClipMarks({ item }: { item: AnimationItem }) {
  const { clip } = item;
  return (
    <span className="flex flex-wrap gap-1">
      <span className={MARK}>
        <Icon name={clip.rootMotion ? 'path' : 'map-pin'} size={11} />
        {clip.rootMotion ? 'root' : 'in place'}
      </span>
      {clip.loops ? (
        <span className={MARK}>
          <Icon name="repeat" size={11} />
          loop
        </span>
      ) : null}
    </span>
  );
}

/**
 * One clip in the grid: its skeleton, source rig, name, length and marks.
 * The button lies over the card, named after the clip (see `CARD_BUTTON`).
 */
export default function ClipCard({ item, selected, onSelect }: ClipCardProps) {
  const { clip } = item;
  return (
    <div
      className={cn(
        CARD,
        'flex flex-col overflow-hidden',
        selected && 'border-clay-line ring-2 ring-clay/25'
      )}
    >
      <button
        type="button"
        aria-pressed={selected}
        aria-label={clip.name}
        title={clip.name}
        onClick={onSelect}
        className={CARD_BUTTON}
      />
      <span className="relative">
        <ClipPose bones={clip.rig.bones} className="aspect-[4/3] w-full border-b border-line" />
        <span className="absolute top-2 left-2 rounded-pill bg-app/85 px-1.5 text-[11px] leading-[18px] font-medium text-fg-dim">
          {RIG_LABEL[clip.rig.kind]}
        </span>
      </span>
      <span className="flex flex-col gap-1.5 p-3">
        <span className="truncate font-semibold">{clip.name}</span>
        <span className="font-mono text-[11.5px] leading-[14px] text-fg-mute">
          {lengthLine(clip)}
        </span>
        <ClipMarks item={item} />
      </span>
    </div>
  );
}
