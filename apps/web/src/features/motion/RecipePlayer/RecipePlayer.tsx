import { Icon } from '@maquette/ui';
import RecipePreview from '@/features/viewer/RecipePreview';
import { checkLines } from '@/state/recipe-review';
import { RecipeLanes } from '../RecipeLanes';
import { PoseKeysBar } from './PoseKeysBar';
import { usePoseGizmoProps } from './pose-gizmo';
import { useRecipePlayer } from './RecipePlayer.hooks';

/**
 * The rough preview over the recipe's timeline: play, pause and a scrub over
 * the bake's frames, the base edits and the changes as lanes on one ruler,
 * and the live checks.
 */
export default function RecipePlayer() {
  const p = useRecipePlayer();
  const gizmo = usePoseGizmoProps();
  const lines = checkLines(p.checks, p.layers).map(line => (
    <li key={line} className="flex items-start gap-1.5 text-meta text-amber">
      <Icon name="warning" size={14} className="mt-0.5 flex-none" />
      {line}
    </li>
  ));
  return (
    <section aria-label="Preview" className="flex min-w-0 flex-col gap-2">
      <RecipePreview
        className="aspect-[4/3] w-full flex-none"
        project={p.project}
        model={p.model}
        clip={p.joined ?? p.clip}
        layers={p.layers}
        markers={p.markers}
        play={{ share: p.share, playing: p.playing, onTick: p.tick }}
        onChecks={p.setChecks}
        laid={{
          keys: p.keysOn ? { on: p.keysOn, gizmo } : null,
          pins: p.pins,
          grips: p.grips,
        }}
      />
      <PoseKeysBar />
      <RecipeLanes frame={p.frame} playing={p.playing} onToggle={p.toggle} onScrub={p.scrub} />
      {lines.length ? (
        <ul aria-label="Preview checks" className="flex list-none flex-col gap-1">
          {lines}
        </ul>
      ) : null}
    </section>
  );
}
