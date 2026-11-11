import { Icon, cn } from '@maquette/ui';
import type { RecipeSheetFrame } from '@/ipc/client';
import { useSheetStills } from './RecipeSheet.hooks';

export interface RecipeSheetProps {
  /** The staged model, or null when it cannot be shown. */
  url: string | null;
  clip: string;
  /** The bake's last frame. */
  last: number;
  frames: readonly RecipeSheetFrame[];
  /** The frame the player stands on, which its tile marks. */
  current: number;
  onPick: (frame: number) => void;
}

/**
 * The review's pose sheet: the bake at the frames the recipe says are worth
 * a look (seams, keys, holds), each a still of the staged model with what is
 * there and what a check saw. Picking one moves the player to its frame.
 */
export default function RecipeSheet({
  url,
  clip,
  last,
  frames,
  current,
  onPick,
}: RecipeSheetProps) {
  const stills = useSheetStills(
    url,
    clip,
    frames.map(f => f.frame),
    last
  );
  if (!frames.length) return null;
  const tiles = frames.map((f, i) => {
    const still = stills?.[i] ?? null;
    return (
      <li key={`${f.frame}-${f.label}`} className="flex-none">
        <button
          type="button"
          aria-pressed={current === f.frame}
          title={`Show frame ${f.frame}`}
          onClick={() => onPick(f.frame)}
          className={cn(
            'flex w-[132px] cursor-pointer flex-col overflow-hidden rounded-card border border-line bg-raised text-left hover:border-line-strong',
            current === f.frame && 'border-line-strong ring-1 ring-line-strong',
            f.warn && 'border-amber/50'
          )}
        >
          <span className="grid aspect-square w-full place-items-center bg-studio">
            {still ? (
              <img src={still} alt="" className="h-full w-full object-cover" />
            ) : (
              <span className="font-mono text-[11.5px] text-fg-mute">
                {stills ? `frame ${f.frame}` : 'Drawing...'}
              </span>
            )}
          </span>
          <span className="flex flex-col gap-0.5 px-2 py-1.5">
            <span className="flex items-center gap-1 text-[11.5px] font-medium text-fg">
              {f.warn ? (
                <Icon
                  name="warning"
                  size={12}
                  className="flex-none text-amber"
                  title="Worth a look"
                />
              ) : null}
              <span className="truncate">
                Frame {f.frame}, {f.label}
              </span>
            </span>
            {f.detail ? (
              <span className={cn('truncate text-[11px] text-fg-mute', f.warn && 'text-amber')}>
                {f.detail}
              </span>
            ) : null}
          </span>
        </button>
      </li>
    );
  });
  return (
    <section aria-label="Pose sheet" className="flex flex-col gap-1.5">
      <div className="flex items-baseline gap-2">
        <h2 className="text-meta font-semibold text-fg">Pose sheet, at seams, keys and holds</h2>
        <span className="truncate text-meta text-fg-mute">
          Frames picked from the recipe, not every 10
        </span>
      </div>
      <ul className="m-0 flex list-none gap-2 overflow-x-auto p-0 pb-1">{tiles}</ul>
    </section>
  );
}
