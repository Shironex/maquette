/**
 * Frostveil's demo of pose keys (`?library=demo`): a slash blended into a
 * block with a pose key change laid on top, the chest leaning into the cut
 * and the sword arm lifted, and the head keyed on the block, so its key
 * rides with that clip.
 */
import type { AnimationClip, RecipeFile } from '../client';
import { baseOf, layer, newRecipe } from './recipe-files';

export const POSE_KEYS_DEMO = 'slash-with-a-lean';

export function poseKeysRecipe(clips: readonly AnimationClip[]): RecipeFile | null {
  const named = (name: string) => clips.find(c => c.name === name);
  const slash = named('Sword And Shield Slash');
  const block = named('Sword And Shield Block');
  if (!slash || !block) return null;
  const recipe = newRecipe('Slash with a lean', baseOf(slash));
  recipe.version = 2;
  recipe.joined = [
    { ...baseOf(block), blend: { frames: 6, curve: 'smooth', lineUp: 'hips-and-facing' } },
  ];
  recipe.layers = [
    {
      ...layer('l-1', 'pose', '', {}),
      mode: 'add',
      keys: [
        { clip: 0, frame: 6, ease: 'smooth', pose: { chest: [0, 0, 0] } },
        {
          clip: 0,
          frame: 12,
          ease: 'out',
          pose: { rightUpperArm: [0, -10, 25] },
        },
        { clip: 0, frame: 20, ease: 'out', pose: { chest: [12, 18, 0] } },
        { clip: 0, frame: 30, ease: 'smooth', pose: { rightUpperArm: [0, 0, 0] } },
        { clip: 0, frame: 40, ease: 'smooth', pose: { chest: [0, 0, 0] } },
        { clip: 1, frame: 10, ease: 'in', pose: { head: [-8, -15, 0] } },
      ],
    },
  ];
  return recipe;
}
