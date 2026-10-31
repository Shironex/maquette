/**
 * Both skeletons at rest in mock mode, as the core reads them from the
 * files: the clip's bones stand as the library has them, the model's mapped
 * bones stand on them with the arms lowered and the hips a little lower,
 * and the bones only on the model hang off the bone their name points at.
 */
import type {
  AnimationClip,
  BonePair,
  FollowChoice,
  OnlyBone,
  RestBone,
  RestCompare,
} from '../client';

const ARM_DROP = (47 * Math.PI) / 180;
const HIPS_SCALE = 0.94;

const HOME: readonly [RegExp, string][] = [
  [/hand(index|middle|ring|pinky|thumb)|^sleeve_l/i, 'LeftForeArm'],
  [/^sleeve_r/i, 'RightForeArm'],
  [/^(cloak|skirt)/i, 'Hips'],
  [/^(hair|eye|jaw)/i, 'Head'],
];

/** The joint above `name`: the one before it in a numbered chain, or where its name belongs. */
function parentOf(name: string, joints: readonly string[], map: readonly BonePair[]) {
  const chain = /^(.*?)(\d+)$/.exec(name);
  if (chain) {
    const before = `${chain[1]}${String(Number(chain[2]) - 1).padStart(chain[2]!.length, '0')}`;
    if (joints.includes(before)) return before;
  }
  const home = HOME.find(([re]) => re.test(name))?.[1] ?? 'Hips';
  return map.find(p => p.bone === home)?.target ?? null;
}

export function onlyBones(
  joints: readonly string[],
  map: readonly BonePair[],
  saved: readonly FollowChoice[]
): OnlyBone[] {
  const driven = map.flatMap(p => (p.target ? [p.target] : []));
  return joints
    .filter(j => !driven.includes(j))
    .map(bone => {
      let at = parentOf(bone, joints, map);
      while (at && !driven.includes(at)) at = parentOf(at, joints, map);
      return {
        bone,
        carriedBy: at,
        follows: saved.find(c => c.bone === bone)?.follows ?? null,
        free: true,
      };
    });
}

type Point = [number, number, number];

function lowered(p: readonly number[], side: number, pivot: readonly number[]): Point {
  const dx = (p[0] ?? 0) - (pivot[0] ?? 0);
  const dy = (p[1] ?? 0) - (pivot[1] ?? 0);
  const [c, s] = [Math.cos(ARM_DROP), Math.sin(ARM_DROP)];
  return [
    (pivot[0] ?? 0) + dx * c + side * dy * s,
    (pivot[1] ?? 0) - Math.abs(dx) * s + dy * c,
    p[2] ?? 0,
  ];
}

export function restCompare(
  clip: AnimationClip | undefined,
  joints: readonly string[],
  map: readonly BonePair[]
): RestCompare | null {
  if (!clip?.rig.bones.length || !joints.length) return null;
  const bySource = new Map(clip.rig.bones.map(b => [b.name, b]));
  const pairOf = (name: string) => map.find(p => p.target === name);
  const placed = new Map<string, { head: Point; tail: Point }>();
  const pivotOf = (bone: string) => {
    const side = bone.startsWith('Left') ? 'LeftArm' : 'RightArm';
    const source = map.find(p => p.bone === side)?.source;
    return source ? (bySource.get(source)?.head ?? null) : null;
  };
  for (const pair of map) {
    const from = pair.source ? bySource.get(pair.source) : undefined;
    if (!pair.target || !from) continue;
    const down = /^(Left|Right)(Arm|ForeArm|Hand)$/.test(pair.bone) ? pivotOf(pair.bone) : null;
    const side = pair.bone.startsWith('Left') ? 1 : -1;
    const at = (p: readonly (number | null)[]): Point => {
      const point = p.map(v => v ?? 0);
      return down
        ? lowered(
            point,
            side,
            down.map(v => v ?? 0)
          )
        : [point[0] ?? 0, point[1] ?? 0, point[2] ?? 0];
    };
    placed.set(pair.target, { head: at(from.head), tail: at(from.tail) });
  }
  const model: RestBone[] = joints.map((name, i) => {
    const parent = pairOf(name) ? null : parentOf(name, joints, map);
    const above = parent ? placed.get(parent) : undefined;
    const home = above?.tail ?? [0, 1, 0];
    const known = placed.get(name) ?? {
      head: home,
      tail: [home[0] + 0.02 * ((i % 3) - 1), home[1] - 0.06, home[2]] as Point,
    };
    placed.set(name, known);
    return { name, parent, head: known.head, tail: known.tail };
  });
  const turn = (bone: string) => ({ bone, deg: 47 });
  const hips = clip.rig.hipsHeight ?? 1;
  return {
    clip: clip.rig.bones,
    model,
    clipShape: 't-pose',
    modelShape: 'a-pose',
    clipArmsDeg: 0,
    modelArmsDeg: 47,
    clipHips: hips,
    modelHips: hips * HIPS_SCALE,
    hipsScale: HIPS_SCALE,
    turns: [
      turn('LeftArm'),
      turn('RightArm'),
      { bone: 'Spine2', deg: 3 },
      { bone: 'LeftShoulder', deg: 6 },
      { bone: 'RightShoulder', deg: 6 },
      { bone: 'LeftUpLeg', deg: 2 },
      { bone: 'RightUpLeg', deg: 2 },
    ],
  };
}
