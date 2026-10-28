/**
 * How mock mode reads bone and clip names the way the core does: the
 * humanoid bones in Mixamo names, Tripo's names for them, and a clip's
 * snake_case name in a model with its pack's words dropped.
 */

export const HUMANOID = [
  'Hips',
  'Spine',
  'Spine1',
  'Spine2',
  'Neck',
  'Head',
  'LeftShoulder',
  'LeftArm',
  'LeftForeArm',
  'LeftHand',
  'RightShoulder',
  'RightArm',
  'RightForeArm',
  'RightHand',
  'LeftUpLeg',
  'LeftLeg',
  'LeftFoot',
  'LeftToeBase',
  'RightUpLeg',
  'RightLeg',
  'RightFoot',
  'RightToeBase',
];

const TRIPO: Record<string, string> = {
  Hips: 'Hip',
  Spine: 'Waist',
  Spine1: 'Spine01',
  Spine2: 'Spine02',
  Neck: 'NeckTwist01',
  Head: 'Head',
  LeftShoulder: 'L_Clavicle',
  LeftArm: 'L_Upperarm',
  LeftForeArm: 'L_Forearm',
  LeftHand: 'L_Hand',
  LeftUpLeg: 'L_Thigh',
  LeftLeg: 'L_Calf',
  LeftFoot: 'L_Foot',
  LeftToeBase: 'L_ToeBase',
};

const tripoName = (bone: string) =>
  TRIPO[bone] ?? TRIPO[bone.replace('Right', 'Left')]?.replace('L_', 'R_') ?? null;

export function boneIn(names: readonly string[], bone: string) {
  const exact = names.find(n => n.split(':').pop() === bone);
  if (exact) return { name: exact, how: 'exact' as const };
  const tripo = tripoName(bone);
  const byName = tripo ? names.find(n => n === tripo) : undefined;
  return byName ? { name: byName, how: 'by-name' as const } : null;
}

const STEM = /\b(standing|pro|pack)\b/gi;

export function defaultName(name: string, pack: string): string {
  const packWords = pack.toLowerCase().split(/[^a-z0-9]+/);
  const words = name
    .replace(STEM, ' ')
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
  const kept = words.filter((w, i) => i === 0 || !packWords.includes(w));
  return kept.join('_') || words.join('_');
}

export const snake = (s: string) =>
  s
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean)
    .join('_');
