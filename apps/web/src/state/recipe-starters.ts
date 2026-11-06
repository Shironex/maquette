/**
 * The starter recipes as the Animations library shows them: built into the
 * app, read only, each naming its base clip by name or running on the rest
 * pose. Pure helpers; the list lives in use-recipe-library-store.ts.
 */
import type { RecipeLayer, RecipeStarter } from '@/ipc/client';
import { counted } from '@/lib/format';
import { boneWords } from './recipe-bones';
import { KIND_WORDS, num } from './recipe-draft';

/** What every starter id starts with; no library recipe id can hold it. */
export const STARTER_PREFIX = 'starter:';

export const isStarterId = (id: string | null | undefined): id is string =>
  !!id?.startsWith(STARTER_PREFIX);

/** "Rest pose · 2 changes" or "Idle · 1 change · 1 blank". */
export function starterLine(s: RecipeStarter): string {
  const parts = [s.baseName, counted(s.layers.length, 'change')];
  if (s.blanks.length) parts.push(counted(s.blanks.length, 'blank'));
  return parts.join(' · ');
}

/** Where a starter's base stands on this computer, in a sentence, and whether it needs a clip. */
export function starterBase(s: RecipeStarter): { text: string; missing: boolean } {
  if (s.rest)
    return { text: "Runs on the model's rest pose, so it needs no clip.", missing: false };
  if (s.base.status === 'found')
    return { text: `Runs on ${s.baseName} from the clip library.`, missing: false };
  const names = s.clips.join(', ');
  return {
    text: `Runs on a clip called ${names}. None is in the clip library yet: a copy still works, and waits for the clip before it bakes.`,
    missing: true,
  };
}

/** "Reach · Right hand → prop · 20% to 80%": a starter's clip is not known, so its range reads as shares. */
export function starterChangeLine(layer: RecipeLayer): string {
  const s = layer.settings;
  const aim = s.target ? ` → ${s.target}` : '';
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const span = `${pct(num(s.start))} to ${pct(num(s.end, 1))}`;
  return [`${KIND_WORDS[layer.kind]} · ${boneWords(layer.bone)}${aim}`, span].join(' · ');
}
