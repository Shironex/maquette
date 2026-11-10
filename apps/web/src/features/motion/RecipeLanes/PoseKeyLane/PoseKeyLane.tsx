import { cn } from '@maquette/ui';
import { boneWords } from '@/state/recipe-bones';
import { EASE_WORDS } from '@/state/recipe-keys';
import { at } from '@/state/recipe-lanes';
import { usePoseKeyLane } from './PoseKeyLane.hooks';

export interface PoseKeyLaneProps {
  /** The pose key change. */
  id: string;
  /** One bone's keys, or null for every key of the change. */
  bone: string | null;
  /** The bake's last frame. */
  frames: number;
}

const DIAMOND =
  'absolute top-1/2 size-2.5 -translate-x-1/2 -translate-y-1/2 rotate-45 rounded-[2px] border focus-visible:outline-2 focus-visible:outline-focus';

/** A lane of key diamonds: drag one to move its key, Alt drag to copy it. */
export default function PoseKeyLane({ id, bone, frames }: PoseKeyLaneProps) {
  const l = usePoseKeyLane(id, bone, frames);
  const what = bone ? boneWords(bone) : 'Pose';
  const diamonds = l.marks.map(m => {
    const f = m.frame ?? 0;
    const here = f === l.frame;
    return (
      <button
        key={`${m.spot.clip}:${m.spot.frame}`}
        type="button"
        aria-label={`${what} key at frame ${f}, eases ${EASE_WORDS[m.ease].toLowerCase()}`}
        title={
          bone
            ? `${boneWords(bone)}, frame ${f}`
            : `${m.bones.map(boneWords).join(', ')}, frame ${f}`
        }
        onPointerDown={l.down(m)}
        onPointerMove={l.move}
        onPointerUp={l.up}
        onKeyDown={l.key(m)}
        className={cn(
          DIAMOND,
          'cursor-grab touch-none',
          bone ? 'border-clay bg-clay' : 'border-fg-dim bg-fg-dim',
          here && 'size-3 border-fg ring-2 ring-clay/50'
        )}
        style={{ left: at(f, frames) }}
      />
    );
  });
  const ghost = l.ghost ? (
    <i
      aria-hidden
      className={cn(DIAMOND, 'pointer-events-none border-dashed border-fg bg-transparent')}
      style={{ left: at(l.ghost.to, frames) }}
    />
  ) : null;
  return (
    <>
      {diamonds}
      {ghost}
    </>
  );
}
