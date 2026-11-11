/**
 * The starter recipes as the Animations library shows them: built into the
 * app, read only, each naming its base clip by name or running on the rest
 * pose. Pure helpers; the list lives in use-recipe-library-store.ts.
 */
import type { RecipeLayer, RecipeStarter } from '@/ipc/client';
import { counted } from '@/lib/format';
import { boneWords } from './recipe-bones';
import { KIND_WORDS, num } from './recipe-draft';
import { handsOfPose } from './recipe-grips';

/** What every starter id starts with; no library recipe id can hold it. */
export const STARTER_PREFIX = 'starter:';

export const isStarterId = (id: string | null | undefined): id is string =>
  !!id?.startsWith(STARTER_PREFIX);

/**
 * "Rest pose · 2 changes", "Idle · 1 change · 1 blank" or, for a combat
 * starter, "Sword And Shield Slash + 2 clips · 2 changes · left handed".
 */
export function starterLine(s: RecipeStarter): string {
  const joined = s.joined?.length ?? 0;
  const parts = [
    joined ? `${s.baseName} + ${counted(joined, 'clip')}` : s.baseName,
    counted(s.layers.length, 'change'),
  ];
  if (s.blanks.length) parts.push(counted(s.blanks.length, 'blank'));
  if (s.mirror) parts.push('left handed');
  return parts.join(' · ');
}

/** Every clip a starter runs on, in order: its base, then what it joins. */
export const starterClips = (s: RecipeStarter): string[] => [s.baseName, ...(s.joined ?? [])];

/** "Hold over on clip 1, frame 15 for 12 f": a starter's time edits, a line each. */
export const starterTimeLines = (s: RecipeStarter): string[] =>
  (s.time ?? []).map(t => {
    const what = { retime: 'Retime', freeze: 'Freeze', 'hold-over': 'Hold over' }[t.kind];
    const where =
      t.kind === 'retime' ? `frames ${t.from} to ${t.to}` : `frame ${t.at} for ${t.frames} f`;
    return `${what} on clip ${t.clip + 1}, ${where}`;
  });

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
  if (layer.kind === 'pose')
    return `${KIND_WORDS.pose} · ${counted(layer.keys?.length ?? 0, 'key')}`;
  if (layer.kind === 'grip') {
    const hands = handsOfPose(layer.pose ?? {});
    return `${KIND_WORDS.grip} · ${hands === 'both' ? 'both hands' : boneWords(layer.bone)}`;
  }
  const s = layer.settings;
  const aim = s.target ? ` → ${s.target}` : '';
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const span = `${pct(num(s.start))} to ${pct(num(s.end, 1))}`;
  return [`${KIND_WORDS[layer.kind]} · ${boneWords(layer.bone)}${aim}`, span].join(' · ');
}
