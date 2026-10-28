import { cn } from '@maquette/ui';
import type { AnimationItem } from '@/ipc/client';
import { RIG_LABEL, lengthLine } from '@/state/animations';
import { ClipMarks } from '../ClipCard';
import { ClipPose } from '../ClipPose';

export interface ClipRowProps {
  item: AnimationItem;
  selected: boolean;
  onSelect: () => void;
}

/** One clip in the list: a small skeleton, name, pack, rig, length and marks. */
export default function ClipRow({ item, selected, onSelect }: ClipRowProps) {
  const { clip } = item;
  return (
    <li>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={clip.name}
        onClick={onSelect}
        className={cn(
          'flex w-full cursor-pointer items-center gap-3 rounded-control px-2 py-1.5 text-left hover:bg-hover',
          selected && 'bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <ClipPose
          bones={clip.rig.bones}
          className="size-10 flex-none rounded-[6px] border border-line"
        />
        <span className="min-w-0 flex-1">
          <b className="block truncate font-semibold">{clip.name}</b>
          <span className="block truncate text-meta text-fg-mute">
            {clip.pack} · {RIG_LABEL[clip.rig.kind]}
          </span>
        </span>
        <span className="font-mono text-[11.5px] text-fg-mute">{lengthLine(clip)}</span>
        <span className="w-[150px]">
          <ClipMarks item={item} />
        </span>
      </button>
    </li>
  );
}
