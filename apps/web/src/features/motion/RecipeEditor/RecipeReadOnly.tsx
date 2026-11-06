import { Button, InlineBanner } from '@maquette/ui';
import type { RecipeBase, RecipeDoc, RecipeLayer, TimeEdit } from '@/ipc/client';
import { counted } from '@/lib/format';
import { boneWords } from '@/state/recipe-bones';
import { KIND_WORDS } from '@/state/recipe-draft';

const TIME_WORDS: Record<TimeEdit['kind'], string> = {
  retime: 'Retime',
  freeze: 'Freeze',
  'hold-over': 'Hold over',
};

function clipLine(clip: RecipeBase, index: number): string {
  const trim = clip.trim ? `keeps ${clip.trim[0]} to ${clip.trim[1]}` : null;
  const blend = index > 0 && clip.blend ? `blend ${clip.blend.frames} f` : null;
  return [`${index + 1}. ${clip.name}`, `${clip.frames} f`, trim, blend]
    .filter(Boolean)
    .join(' · ');
}

function timeLine(edit: TimeEdit): string {
  const where = `clip ${edit.clip + 1}`;
  if (edit.kind === 'retime')
    return `${TIME_WORDS.retime} ${where}, ${edit.from ?? '?'} to ${edit.to ?? '?'} at ${edit.speed ?? 1}x`;
  return `${TIME_WORDS[edit.kind]} ${where}, frame ${edit.at ?? '?'} for ${edit.frames ?? '?'} f`;
}

function layerLine(layer: RecipeLayer): string {
  const word = KIND_WORDS[layer.kind];
  if (layer.kind === 'pose') return `${word} · ${counted(layer.keys?.length ?? 0, 'key')}`;
  return `${word} · ${boneWords(layer.bone)}`;
}

function Section({ label, lines }: { label: string; lines: string[] }) {
  if (!lines.length) return null;
  const items = lines.map((line, i) => <li key={`${i}-${line}`}>{line}</li>);
  return (
    <section aria-label={label} className="flex flex-col gap-1">
      <h3 className="text-meta font-semibold text-fg-dim">{label}</h3>
      <ul className="m-0 flex list-none flex-col gap-0.5 p-0 font-mono text-[12px]">{items}</ul>
    </section>
  );
}

/**
 * A recipe of file version 2 (joined clips, trims, mirror, time edits,
 * pins, pose keys or grips), shown read only until the editor and the bake
 * learn those: what it holds, with no Save and no Apply.
 */
export default function RecipeReadOnly({ doc, onBack }: { doc: RecipeDoc; onBack: () => void }) {
  const { recipe } = doc;
  const clips = [recipe.base, ...(recipe.joined ?? [])];
  return (
    <div
      aria-label={`Recipe ${recipe.name}`}
      role="region"
      className="flex min-h-0 flex-1 flex-col gap-3"
    >
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="ghost" icon="arrow-left" onClick={onBack}>
          Recipes
        </Button>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-panel-title">{recipe.name}</h2>
          <p className="truncate font-mono text-[11.5px] text-fg-mute">
            {counted(clips.length, 'clip')} · {doc.file} · read only
          </p>
        </div>
      </div>
      <InlineBanner
        tone="info"
        title="This recipe uses what this Maquette cannot edit or bake yet"
        detail="It joins clips, trims, mirrors or times them, or holds pins, pose keys or grips. It shows here as it is; an agent can still change it, and a later Maquette edits and bakes it."
      />
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto rounded-card border border-line bg-raised p-3">
        <Section label="Clips" lines={clips.map(clipLine)} />
        <Section label="Mirror" lines={recipe.mirror ? ['Left and right swapped'] : []} />
        <Section label="Time edits" lines={(recipe.time ?? []).map(timeLine)} />
        <Section label="Changes" lines={recipe.layers.map(layerLine)} />
        <Section label="Blanks" lines={recipe.blanks.map(b => `${b.id} · ${b.label}`)} />
      </div>
    </div>
  );
}
