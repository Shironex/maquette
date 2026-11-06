import { cn } from '@maquette/ui';
import type { RecipeLayer } from '@/ipc/client';
import { KIND_WORDS } from '@/state/recipe-draft';
import { across, at } from '@/state/recipe-lanes';
import { useChangeBar } from './ChangeBar.hooks';

export interface ChangeBarProps {
  layer: RecipeLayer;
  /** The bake's last frame. */
  frames: number;
  toFrame: (share: number) => number;
  toShare: (frame: number) => number;
  picked: boolean;
  onPick: () => void;
}

const HANDLE =
  'absolute inset-y-0 w-2 cursor-ew-resize rounded-[3px] bg-fg/70 focus-visible:outline-2 focus-visible:outline-focus';

/** One change as a bar over the bake frames it covers, to move or stretch. */
export default function ChangeBar({
  layer,
  frames,
  toFrame,
  toShare,
  picked,
  onPick,
}: ChangeBarProps) {
  const b = useChangeBar(layer, frames, toFrame, toShare);
  const word = KIND_WORDS[layer.kind];
  return (
    <div
      role="button"
      tabIndex={0}
      aria-label={`${word} ${layer.id}, frames ${b.from} to ${b.to}`}
      aria-pressed={picked}
      title="Drag to move it, or use the arrow keys"
      onClick={onPick}
      onKeyDown={e => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onPick();
        } else b.key('move')(e);
      }}
      onPointerDown={b.down('move')}
      onPointerMove={b.move}
      onPointerUp={b.up}
      className={cn(
        'absolute inset-y-1 flex cursor-grab items-center overflow-hidden rounded-[5px] bg-clay/70 px-2.5 text-[11px] font-medium whitespace-nowrap text-clay-ink focus-visible:outline-2 focus-visible:outline-focus',
        picked && 'outline-2 outline-fg'
      )}
      style={{ left: at(b.from, frames), width: across(b.from, b.to, frames) }}
    >
      <span className="truncate">{word}</span>
      <span
        role="slider"
        tabIndex={0}
        aria-label={`Starts at frame ${b.from}`}
        aria-valuemin={0}
        aria-valuemax={frames}
        aria-valuenow={b.from}
        onKeyDown={b.key('start')}
        onPointerDown={b.down('start')}
        onPointerMove={b.move}
        onPointerUp={b.up}
        onClick={e => e.stopPropagation()}
        className={cn(HANDLE, 'left-0')}
      />
      <span
        role="slider"
        tabIndex={0}
        aria-label={`Ends at frame ${b.to}`}
        aria-valuemin={0}
        aria-valuemax={frames}
        aria-valuenow={b.to}
        onKeyDown={b.key('end')}
        onPointerDown={b.down('end')}
        onPointerMove={b.move}
        onPointerUp={b.up}
        onClick={e => e.stopPropagation()}
        className={cn(HANDLE, 'right-0')}
      />
    </div>
  );
}
