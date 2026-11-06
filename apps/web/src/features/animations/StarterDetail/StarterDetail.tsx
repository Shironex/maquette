import { Button } from '@maquette/ui';
import { MixamoBadge } from '@/features/common/MixamoBadge';
import type { RecipeStarter } from '@/ipc/client';
import { BLANK_WORDS } from '@/state/recipe-draft';
import { starterBase, starterChangeLine, starterLine } from '@/state/recipe-starters';
import { useStarterDetail } from './StarterDetail.hooks';

export interface StarterDetailProps {
  starter: RecipeStarter;
}

const CAPTION = 'text-caption tracking-[0.04em] text-fg-mute uppercase';
const LIST = 'm-0 flex list-none flex-col gap-1 p-0 text-meta text-fg-dim';

/**
 * The selected starter: what it does, the clip it runs on (or the rest
 * pose), its changes and blanks, Use in project and Save to library. It
 * never changes: an edit needs a copy in a project or the library.
 */
export default function StarterDetail({ starter }: StarterDetailProps) {
  const d = useStarterDetail(starter);
  const base = starterBase(starter);
  const changes = starter.layers.map(l => (
    <li key={l.id} className="truncate" title={starterChangeLine(l)}>
      {starterChangeLine(l)}
    </li>
  ));
  const blanks = starter.blanks.map(b => (
    <li key={b.id} className="truncate">
      {BLANK_WORDS[b.kind]} · {b.label}
      {b.kind === 'prop' ? ' (open: pick a prop of the project)' : ''}
    </li>
  ));
  return (
    <aside
      aria-label="Selected starter"
      className="flex w-[320px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-[18px] min-[1280px]:w-[360px]"
    >
      <div>
        <div className="flex items-center gap-2">
          <h2 className="min-w-0 flex-1 truncate text-panel-title">{starter.name}</h2>
          {starter.mixamo ? <MixamoBadge /> : null}
        </div>
        <p className="mt-0.5 font-mono text-[11.5px] text-fg-mute">{starterLine(starter)}</p>
      </div>
      <p className="text-body text-fg-dim">{starter.about}</p>
      <p className={base.missing ? 'text-meta text-amber' : 'text-meta text-fg-mute'}>
        {base.text}
      </p>
      <section aria-label="What it holds" className="flex flex-col gap-1.5">
        <p className={CAPTION}>Changes</p>
        <ul aria-label="Changes" className={LIST}>
          {changes}
        </ul>
        <p className={CAPTION}>Blanks</p>
        {blanks.length ? (
          <ul aria-label="Blanks" className={LIST}>
            {blanks}
          </ul>
        ) : (
          <p className="text-meta text-fg-mute">No blanks.</p>
        )}
      </section>
      <Button
        variant="primary"
        icon="plus"
        tooltip="Copies the starter into a project as a new recipe you can edit"
        onClick={d.use}
      >
        Use in project
      </Button>
      <Button
        icon="books"
        loading={d.copying}
        tooltip="Adds an editable copy to the library as a new recipe at r1"
        onClick={d.copy}
      >
        Save to library
      </Button>
      <p className="text-meta text-fg-mute">
        Starters are built into Maquette and never change. Nothing is written to the library until
        you save one there.
      </p>
    </aside>
  );
}
