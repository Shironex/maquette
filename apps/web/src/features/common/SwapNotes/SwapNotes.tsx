import { Icon, cn } from '@maquette/ui';
import { useSwapNotes } from './SwapNotes.hooks';

export interface SwapNotesProps {
  project: string;
  /** Changes when the list may have changed: a run that ended, a plan that reloaded. */
  refresh?: string | number | null;
  className?: string;
}

/**
 * The models a built asset replaced since the last Ship, for whoever looks
 * at what the next Ship writes: the asset, the file that takes the place of
 * which, and the triangles and bytes of both models. Nothing when no swap
 * waits.
 */
export default function SwapNotes({ project, refresh = null, className }: SwapNotesProps) {
  const lines = useSwapNotes(project, refresh);
  if (lines.length === 0) return null;
  const items = lines.map(line => (
    <li key={line} className="flex items-start gap-2">
      <Icon name="arrows-left-right" size={14} className="mt-0.5 flex-none text-amber" />
      <span className="min-w-0">{line}</span>
    </li>
  ));
  return (
    <section
      aria-label="Built models that replace another model in this Ship"
      className={cn('rounded-card border border-amber/40 bg-panel px-3.5 py-3', className)}
    >
      <h2 className="pb-1.5 text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
        Replaced by a built model <span className="ml-1 tabular-nums">{lines.length}</span>
      </h2>
      <ul className="m-0 flex list-none flex-col gap-1.5 p-0 text-[12.5px] text-fg-dim">{items}</ul>
    </section>
  );
}
