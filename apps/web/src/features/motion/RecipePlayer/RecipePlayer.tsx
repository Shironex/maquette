import { Icon, IconButton } from '@maquette/ui';
import RecipePreview from '@/features/viewer/RecipePreview';
import { KIND_WORDS, num } from '@/state/recipe-draft';
import { checkLines } from '@/state/recipe-review';
import { useRecipePlayer } from './RecipePlayer.hooks';

const pct = (share: number) => `${Math.min(Math.max(share, 0), 1) * 100}%`;

/**
 * The rough preview with play, pause and a scrub over the clip, each change
 * drawn as a lane over the part of the clip it covers, and the live checks.
 */
export default function RecipePlayer() {
  const p = useRecipePlayer();
  const lanes = p.layers.map(l => (
    <div key={l.id} className="relative h-2.5" title={`${KIND_WORDS[l.kind]} ${l.id}`}>
      <span
        className="absolute inset-y-0 rounded-pill bg-clay/70"
        style={{
          left: pct(num(l.settings.start)),
          width: pct(num(l.settings.end, 1) - num(l.settings.start)),
        }}
      />
    </div>
  ));
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
        clip={p.clip}
        layers={p.layers}
        markers={p.markers}
        share={p.share}
        playing={p.playing}
        onTick={p.tick}
        onChecks={p.setChecks}
      />
      <div className="flex flex-col gap-1.5 rounded-card border border-line bg-panel px-2.5 py-2">
        <div className="flex items-center gap-2">
          <IconButton
            icon={p.playing ? 'pause' : 'play'}
            label={p.playing ? 'Pause' : 'Play'}
            size="sm"
            onClick={p.toggle}
          />
          <input
            type="range"
            aria-label="Playhead"
            min={0}
            max={p.frames}
            step={1}
            value={p.frame}
            onChange={e => p.scrub(Number(e.target.value))}
            className="min-w-0 flex-1 accent-clay"
          />
          <span className="w-[86px] flex-none text-right font-mono text-[11.5px] text-fg-dim">
            frame {p.frame} / {p.frames}
          </span>
        </div>
        {lanes.length ? (
          <div aria-hidden className="relative ml-9 mr-[94px] flex flex-col gap-1">
            {lanes}
            <i className="absolute -inset-y-0.5 w-px bg-fg" style={{ left: pct(p.share) }} />
          </div>
        ) : null}
      </div>
      {lines.length ? (
        <ul aria-label="Preview checks" className="flex list-none flex-col gap-1">
          {lines}
        </ul>
      ) : null}
    </section>
  );
}
