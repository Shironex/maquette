import { Button, InlineBanner, Loader, cn } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { MixamoBadge } from '@/features/common/MixamoBadge';
import type { LibraryRecipeDoc, LibraryRecipeSummary } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { whenOf } from '@/state/plan-log';
import { BLANK_WORDS, layerLine } from '@/state/recipe-draft';
import { libraryLine, revisionSource } from '@/state/recipe-library';
import { useLibraryRecipeDetail } from './LibraryRecipeDetail.hooks';

export interface LibraryRecipeDetailProps {
  summary: LibraryRecipeSummary;
}

const CAPTION = 'text-caption tracking-[0.04em] text-fg-mute uppercase';

/** What a revision holds: its changes in the order they are laid over the clip, and its blanks. */
function Holds({ doc }: { doc: LibraryRecipeDoc }) {
  const { recipe } = doc;
  const changes = recipe.layers.map(l => (
    <li key={l.id} className="truncate" title={layerLine(l, recipe.base.frames)}>
      {layerLine(l, recipe.base.frames)}
    </li>
  ));
  const blanks = recipe.blanks.map(b => (
    <li key={b.id} className="truncate">
      {BLANK_WORDS[b.kind]} · {b.label}
    </li>
  ));
  const list = 'm-0 flex list-none flex-col gap-1 p-0 text-meta text-fg-dim';
  return (
    <>
      <p className="text-meta text-fg-mute">
        On {recipe.base.name}, {recipe.base.frames} frames
      </p>
      {doc.base.warning ? <p className="text-meta text-amber">{doc.base.warning}</p> : null}
      <p className={CAPTION}>Changes</p>
      {changes.length ? (
        <ul aria-label="Changes" className={list}>
          {changes}
        </ul>
      ) : (
        <p className="text-meta text-fg-mute">No changes.</p>
      )}
      <p className={CAPTION}>Blanks</p>
      {blanks.length ? (
        <ul aria-label="Blanks" className={list}>
          {blanks}
        </ul>
      ) : (
        <p className="text-meta text-fg-mute">No blanks.</p>
      )}
    </>
  );
}

/**
 * The selected library recipe: every revision newest first with where it
 * came from, what the picked one holds, Use in project, Export and the
 * removals, each asked first.
 */
export default function LibraryRecipeDetail({ summary }: LibraryRecipeDetailProps) {
  const d = useLibraryRecipeDetail(summary);
  const now = clock.now();
  const revisions = d.revisions.map(r => (
    <li key={r.revision}>
      <button
        type="button"
        aria-pressed={r.revision === d.revision}
        onClick={() => d.pick(r.revision)}
        className={cn(
          'flex w-full cursor-pointer items-baseline gap-2 rounded-control px-2 py-1.5 text-left hover:bg-hover',
          r.revision === d.revision && 'bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <b className="w-7 flex-none font-mono text-[12px] font-semibold">r{r.revision}</b>
        <span className="min-w-0 flex-1 truncate text-meta text-fg-dim">
          {revisionSource(r, d.projectName)}
        </span>
        <span className="flex-none text-meta text-fg-mute">{whenOf(r.savedAt, now)}</span>
      </button>
    </li>
  ));
  const one = summary.revisions.length === 1;
  return (
    <aside
      aria-label="Selected recipe"
      className="flex w-[320px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel p-[18px] min-[1280px]:w-[360px]"
    >
      <div>
        <div className="flex items-center gap-2">
          <h2 className="min-w-0 flex-1 truncate text-panel-title">{summary.name}</h2>
          {summary.mixamo ? <MixamoBadge /> : null}
        </div>
        {summary.problem ? null : (
          <p className="mt-0.5 font-mono text-[11.5px] text-fg-mute">{libraryLine(summary)}</p>
        )}
      </div>
      {summary.problem ? (
        <InlineBanner tone="danger" title="It could not be read" detail={summary.problem} />
      ) : null}
      <section aria-label="Revisions" className="flex flex-col gap-1.5">
        <p className={CAPTION}>Revisions</p>
        <ul className="m-0 flex list-none flex-col gap-0.5 p-0">{revisions}</ul>
      </section>
      {summary.latest > 0 ? (
        <section aria-label={`What r${d.revision} holds`} className="flex flex-col gap-1.5">
          {d.doc ? (
            <Holds doc={d.doc} />
          ) : d.error ? (
            <p role="alert" className="text-meta text-danger">
              {d.error}
            </p>
          ) : (
            <Loader label={`Reading r${d.revision}...`} />
          )}
        </section>
      ) : null}
      <Button
        variant="primary"
        icon="plus"
        disabled={!d.doc}
        disabledReason="Wait for the revision to be read"
        tooltip={`Copies r${d.revision} into a project as a new recipe`}
        onClick={d.use}
      >
        Use in project
      </Button>
      <Button
        icon="export"
        disabled={summary.latest === 0}
        tooltip={`Saves r${d.revision} as a .recipe.json where you choose`}
        onClick={d.exportFile}
      >
        Export
      </Button>
      {d.refused ? <InlineBanner tone="danger" title="Not removed" detail={d.refused} /> : null}
      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="ghost"
          icon="trash"
          disabled={summary.latest === 0}
          tooltip={
            one
              ? 'The only revision: remove the whole recipe instead'
              : `Moves r${d.revision} aside; the other revisions stay`
          }
          onClick={() => d.ask('revision')}
        >
          Remove r{d.revision}
        </Button>
        <Button size="sm" variant="ghost" icon="trash" onClick={() => d.ask('recipe')}>
          Remove recipe
        </Button>
      </div>
      {d.asking ? (
        <ChoiceDialog
          title={d.asking === 'recipe' ? `Remove ${summary.name}?` : `Remove r${d.revision}?`}
          onDismiss={() => d.ask(null)}
          choices={[
            { label: 'Cancel', onClick: () => d.ask(null) },
            {
              label: d.asking === 'recipe' ? 'Remove recipe' : `Remove r${d.revision}`,
              variant: 'danger',
              loading: d.removing,
              onClick: d.remove,
            },
          ]}
        >
          <p>
            {d.asking === 'recipe'
              ? 'Every revision is moved aside. Project recipes made from it stay, and say their library recipe is gone.'
              : `r${d.revision} of ${summary.name} is moved aside. Project recipes made from it stay as they are.`}
          </p>
        </ChoiceDialog>
      ) : null}
    </aside>
  );
}
