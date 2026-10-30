/**
 * `?avatar=custom`: a character whose rig names its joints the way a
 * person types them ("Left elbow", "ThumbProximal_L") and has two joints
 * for each upper leg ("Left leg" and "Left thigh"). The reader guesses the
 * rest from the descriptive names but leaves the upper legs for a person
 * to assign in the Bone map. The report follows the overrides, as the
 * Rust reader does.
 */
import type { BoneNaming, VrmFinding, VrmReadiness } from '../client';
import { vrmReadiness } from './vrm-fixtures';

const SIDES = [
  ['left', 'Left', 'L'],
  ['right', 'Right', 'R'],
] as const;

const FINGERS = ['Thumb', 'Index', 'Middle', 'Ring', 'Little'] as const;

/** The VRM 0 segment names this rig uses, for every finger. */
const OLD_SEGMENTS = ['Proximal', 'Intermediate', 'Distal'] as const;

function vrmSegment(finger: string, i: number) {
  return finger === 'Thumb'
    ? (['Metacarpal', 'Proximal', 'Distal'] as const)[i]!
    : OLD_SEGMENTS[i]!;
}

/** The bones the reader maps on its own, VRM name to joint. */
function autoBones(): Record<string, string> {
  const map: Record<string, string> = {
    hips: 'Hips',
    spine: 'Spine',
    chest: 'Chest',
    neck: 'Neck',
    head: 'Head',
  };
  for (const [s, word, letter] of SIDES) {
    Object.assign(map, {
      [`${s}Shoulder`]: `${word} shoulder`,
      [`${s}UpperArm`]: `${word} arm`,
      [`${s}LowerArm`]: `${word} elbow`,
      [`${s}Hand`]: `${word} wrist`,
      [`${s}LowerLeg`]: `${word} knee`,
      [`${s}Foot`]: `${word} ankle`,
      [`${s}Toes`]: `${word} toe`,
    });
    for (const finger of FINGERS) {
      OLD_SEGMENTS.forEach((segment, i) => {
        map[`${s}${finger}${vrmSegment(finger, i)}`] = `${finger}${segment}_${letter}`;
      });
    }
  }
  return map;
}

/** Every skin joint, the way the rig lists them. */
export function customJoints(): string[] {
  const joints = ['Hips', 'Spine', 'Chest', 'Neck', 'Head', 'Head top end'];
  for (const [, word, letter] of SIDES) {
    joints.push(
      `${word} shoulder`,
      `${word} arm`,
      `${word} elbow`,
      `${word} wrist`,
      `${word} leg`,
      `${word} thigh`,
      `${word} knee`,
      `${word} ankle`,
      `${word} toe`
    );
    for (const finger of FINGERS) {
      for (const segment of OLD_SEGMENTS) joints.push(`${finger}${segment}_${letter}`);
    }
  }
  joints.push('Hair front', 'Hair back', 'Skirt front', 'Skirt back', 'Ribbon');
  return joints;
}

/** Joints whose names hold a body word, as the reader counts them. */
const HUMANOID_JOINTS = 53;
const EXACT = new Set(['hips', 'spine', 'chest', 'neck', 'head']);
const REQUIRED_EXACT = 3;
const UPPER_LEGS = ['leftUpperLeg', 'rightUpperLeg'] as const;

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** The custom rig's readiness with a spec's bone overrides on top. */
export function customRigReadiness(overrides: Partial<Record<string, string | null>>) {
  const mapped = autoBones();
  const naming: Record<string, BoneNaming> = {};
  for (const [bone, joint] of Object.entries(overrides)) {
    if (joint) {
      mapped[bone] = joint;
      naming[bone] = 'override';
    } else delete mapped[bone];
  }
  const by = (bone: string): BoneNaming =>
    naming[bone] ?? (EXACT.has(bone) ? 'vrm' : 'descriptive');
  const descriptive = Object.keys(mapped).filter(b => by(b) === 'descriptive').length;
  const missing = UPPER_LEGS.filter(b => !(b in overrides) && !mapped[b]);
  const ambiguous = missing.map(b => {
    const word = b.startsWith('left') ? 'Left' : 'Right';
    return `2 joints match ${b}: ${word} leg and ${word} thigh.`;
  });
  const unmapped = UPPER_LEGS.filter(b => !mapped[b]);
  const confident = ambiguous.length === 0;
  const findings: VrmFinding[] = [];
  if (unmapped.length > 0) {
    findings.push({
      id: 'required-bones',
      level: 'blocker',
      title: `${plural(unmapped.length, 'bone', 'bones')} VRM needs not found`,
      detail: `No joint maps to ${unmapped.join(' and ')}. Rename the joints, or pick them in the bone map.`,
    });
  }
  const conventionRequired = 15 - REQUIRED_EXACT - UPPER_LEGS.length;
  const parts: string[] = [];
  if (missing.length > 0) {
    parts.push(
      `${HUMANOID_JOINTS} joints look like humanoid bones, but only ${conventionRequired} of the 15 required bones match descriptive names.`
    );
  }
  parts.push(...ambiguous);
  parts.push(
    `${plural(descriptive, 'bone was', 'bones were')} read from descriptive names such as Left elbow, which is a guess.`
  );
  parts.push(
    confident
      ? 'Check them in the Bone map and set any that are wrong.'
      : 'Assign the bones in the Bone map, or have an agent set them with set_avatar. Nothing past this is guessed.'
  );
  findings.push({
    id: 'bone-naming',
    level: 'warning',
    title: confident
      ? `${plural(descriptive, 'bone', 'bones')} matched by descriptive names`
      : 'Bone names are unclear',
    detail: parts.join(' '),
  });
  const count = Object.keys(mapped).length;
  findings.push({
    id: 'bones',
    level: 'note',
    title: `${count} of 55 humanoid bones mapped`,
    detail:
      '30 finger bones among them. Not found, all optional: eyes, jaw. Without eye bones the look at stays off.',
  });
  const readiness: VrmReadiness = vrmReadiness(
    mapped,
    by,
    findings,
    { height: 1.58, feetY: 0.07 },
    {
      naming: {
        convention: 'descriptive',
        conventionRequired,
        missingRequired: missing.length,
        humanoidJoints: HUMANOID_JOINTS,
        descriptive,
        ambiguous,
        confident,
      },
      joints: customJoints(),
    }
  );
  return readiness;
}
