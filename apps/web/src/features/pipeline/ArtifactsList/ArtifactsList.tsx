import { Button } from '@maquette/ui';
import type { ModelFile } from '@/ipc/client';
import { SHOWN, useArtifactsList } from './ArtifactsList.hooks';

export interface ArtifactsListProps {
  models: readonly ModelFile[];
  /** A run is going, so a file without a size is still being written. */
  busy: boolean;
  /** A game writes "files"; a site writes "artifacts". */
  game?: boolean;
}

/** What the last run wrote, each model file raw export to compressed, beside the log. */
export default function ArtifactsList({ models, busy, game = false }: ArtifactsListProps) {
  const { all, toggle, total, rows } = useArtifactsList(models, busy);
  if (total === 0) return null;
  const title = game ? 'Generated files' : 'Generated artifacts';
  const items = rows.map(r => (
    <li key={r.key} className="flex min-w-0 items-baseline gap-2 font-mono text-[12px]">
      <span className="min-w-0 flex-1 truncate">{r.file}</span>
      <span className={r.pending ? 'text-fg-mute' : 'text-fg-dim'}>{r.size}</span>
    </li>
  ));
  return (
    <section
      aria-label={title}
      className="flex flex-none flex-col gap-1.5 rounded-card border border-line bg-panel px-4 py-3"
    >
      <div className="flex items-baseline gap-2">
        <h2 className="text-[13px] leading-5 font-semibold">{title}</h2>
        <span className="text-meta text-fg-mute">
          {game ? 'raw export to game export' : 'raw export to compressed'}
        </span>
        {total > SHOWN ? (
          <Button
            size="sm"
            variant="ghost"
            className="ml-auto"
            aria-expanded={all}
            onClick={toggle}
          >
            {all ? 'Show fewer' : `Show all ${total}`}
          </Button>
        ) : null}
      </div>
      <ul className="grid list-none grid-cols-2 gap-x-6 gap-y-0.5">{items}</ul>
    </section>
  );
}
