import type { AnimationClip, RecipeFile } from '../client';
import { baseOf, layer, newRecipe } from './recipe-files';

export const JOINED_DEMO = 'slash-into-block';

/**
 * Frostveil's demo of joined clips: a slash trimmed and sped up in the
 * middle, blended into a block that freezes on its guard (the library has
 * the block with other keys), then an idle, with a nudge laid over.
 */
export function joinedRecipe(clips: readonly AnimationClip[]): RecipeFile | null {
  const named = (name: string) => clips.find(c => c.name === name);
  const [slash, block, idle] = [
    named('Sword And Shield Slash'),
    named('Sword And Shield Block'),
    named('Sword And Shield Idle'),
  ];
  if (!slash || !block || !idle) return null;
  const recipe = newRecipe('Slash into block', { ...baseOf(slash), trim: [4, 44] });
  recipe.version = 2;
  recipe.joined = [
    {
      ...baseOf(block),
      hash: 'old',
      trim: [2, 28],
      blend: { frames: 6, curve: 'smooth', lineUp: 'hips-and-facing' },
    },
    { ...baseOf(idle), trim: [0, 40], blend: { frames: 8, curve: 'smooth', lineUp: 'hips' } },
  ];
  recipe.time = [
    { kind: 'retime', clip: 0, from: 14, to: 26, speed: 1.6, ease: 2 },
    { kind: 'freeze', clip: 1, at: 20, frames: 12, breathe: 0.4, easeIn: 3, easeOut: 3 },
  ];
  recipe.layers = [
    layer('l-1', 'nudge', 'spine', { start: 0.3, end: 0.7, rotation: [8, 0, 0], weight: 0.8 }),
  ];
  return recipe;
}
