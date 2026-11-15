import type { LibraryRecipeDoc } from '@/ipc/client';
import { BLANK_WORDS, layerLine } from '@/state/recipe-draft';
import { clipsOf } from '@/state/recipe-timeline';
import { clipsWords } from '@/state/recipes';
import { useHoldsTimeline } from './RecipeHolds.hooks';

const CAPTION = 'text-caption tracking-[0.04em] text-fg-mute uppercase';
const LIST = 'm-0 flex list-none flex-col gap-1 p-0 text-meta text-fg-dim';

/**
 * What a revision holds: the clips it joins, its changes in the order they
 * are laid over them (their frames on the whole joined timeline), and its
 * blanks.
 */
export default function RecipeHolds({ doc }: { doc: LibraryRecipeDoc }) {
  const { recipe } = doc;
  const dt = useHoldsTimeline(recipe);
  const joined = recipe.joined?.length ?? 0;
  const clips = joined
    ? clipsOf(recipe).map((c, i) => (
        <li key={`${i}-${c.name}`} className="truncate">
          {i + 1}. {c.name}
        </li>
      ))
    : null;
  const changes = recipe.layers.map(l => {
    const line = layerLine(l, dt.toFrame);
    return (
      <li key={l.id} className="truncate" title={line}>
        {line}
      </li>
    );
  });
  const blanks = recipe.blanks.map(b => (
    <li key={b.id} className="truncate">
      {BLANK_WORDS[b.kind]} · {b.label}
    </li>
  ));
  return (
    <>
      <p className="text-meta text-fg-mute">
        On {clipsWords(recipe.base.name, joined)}, {dt.timeline.frames} frames
      </p>
      {doc.base.warning ? <p className="text-meta text-amber">{doc.base.warning}</p> : null}
      {clips ? (
        <>
          <p className={CAPTION}>Clips, joined in order</p>
          <ul aria-label="Clips" className={LIST}>
            {clips}
          </ul>
        </>
      ) : null}
      <p className={CAPTION}>Changes</p>
      {changes.length ? (
        <ul aria-label="Changes" className={LIST}>
          {changes}
        </ul>
      ) : (
        <p className="text-meta text-fg-mute">No changes.</p>
      )}
      <p className={CAPTION}>Blanks</p>
      {blanks.length ? (
        <ul aria-label="Blanks" className={LIST}>
          {blanks}
        </ul>
      ) : (
        <p className="text-meta text-fg-mute">No blanks.</p>
      )}
    </>
  );
}
