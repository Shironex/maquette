/**
 * VRM readiness as the Rust reader reports it for the mock models, built
 * from the bones each one maps so the 55 bone list stays short here.
 */
import type { BoneNaming, VrmBone, VrmFinding, VrmReadiness } from '../client';

const SIDED = ['UpperLeg', 'LowerLeg', 'Foot', 'Toes', 'Shoulder', 'UpperArm', 'LowerArm', 'Hand'];
const REQUIRED = new Set(['UpperLeg', 'LowerLeg', 'Foot', 'UpperArm', 'LowerArm', 'Hand']);
const FINGERS: [string, string[]][] = [
  ['Thumb', ['Metacarpal', 'Proximal', 'Distal']],
  ['Index', ['Proximal', 'Intermediate', 'Distal']],
  ['Middle', ['Proximal', 'Intermediate', 'Distal']],
  ['Ring', ['Proximal', 'Intermediate', 'Distal']],
  ['Little', ['Proximal', 'Intermediate', 'Distal']],
];

/** Every VRM humanoid bone in the reader's order, with whether VRM requires it. */
function humanBones(): [string, boolean][] {
  const out: [string, boolean][] = [
    ['hips', true],
    ['spine', true],
    ['chest', false],
    ['upperChest', false],
    ['neck', false],
    ['head', true],
    ['leftEye', false],
    ['rightEye', false],
    ['jaw', false],
  ];
  for (const bone of SIDED) {
    for (const side of ['left', 'right']) out.push([`${side}${bone}`, REQUIRED.has(bone)]);
  }
  for (const side of ['left', 'right']) {
    for (const [finger, segments] of FINGERS) {
      for (const segment of segments) out.push([`${side}${finger}${segment}`, false]);
    }
  }
  return out;
}

/** A readiness report from the bones a rig maps (VRM name to joint). */
export function vrmReadiness(
  mapped: Record<string, string>,
  naming: BoneNaming,
  findings: VrmFinding[],
  pose: Partial<VrmReadiness['pose']> = {}
): VrmReadiness {
  const bones: VrmBone[] = humanBones().map(([bone, required]) => ({
    bone,
    required,
    joint: mapped[bone] ?? null,
    naming: mapped[bone] ? naming : null,
  }));
  return {
    ready: !findings.some(f => f.level === 'blocker'),
    mapped: bones.filter(b => b.joint).length,
    required: bones.filter(b => b.required && b.joint).length,
    bones,
    pose: {
      facing: '+z',
      facingDeg: 0,
      sidesSwapped: false,
      armLeftDeg: 0,
      armRightDeg: 0,
      groundY: 0,
      feetY: 0.06,
      hipsOffset: 0,
      hipsSide: 0,
      hipsForward: 0,
      height: 1.2,
      uniformScale: true,
      negativeScale: [],
      unevenScale: [],
      scaled: [],
      scaleNoise: 0,
      asymmetry: 0,
      asymmetryBone: null,
      ...pose,
    },
    findings,
  };
}

/** The clay figure: Unreal names, a clean T-pose, three clips. */
export const CLAY_FIGURE_VRM = vrmReadiness(
  {
    hips: 'hips',
    spine: 'spine_01',
    chest: 'spine_02',
    upperChest: 'spine_03',
    neck: 'neck',
    head: 'head',
    leftUpperLeg: 'thigh_l',
    rightUpperLeg: 'thigh_r',
    leftLowerLeg: 'calf_l',
    rightLowerLeg: 'calf_r',
    leftFoot: 'foot_l',
    rightFoot: 'foot_r',
    leftShoulder: 'clavicle_l',
    rightShoulder: 'clavicle_r',
    leftUpperArm: 'upperarm_l',
    rightUpperArm: 'upperarm_r',
    leftLowerArm: 'lowerarm_l',
    rightLowerArm: 'lowerarm_r',
    leftHand: 'hand_l',
    rightHand: 'hand_r',
  },
  'unreal',
  [
    {
      id: 'bones',
      level: 'note',
      title: '20 of 55 humanoid bones mapped',
      detail:
        'Not found, all optional: eyes, jaw, fingers. Without eye bones the look at stays off.',
    },
    {
      id: 'animations',
      level: 'note',
      title: '3 animations are dropped on export',
      detail: 'VRM carries no clips. Motion ships as VRMA files instead.',
    },
    {
      id: 'scale-noise',
      level: 'note',
      title: '2 node scales are off 1 by float noise',
      detail: 'The VRM writer snaps them to 1.',
    },
  ],
  { groundY: 0.005, height: 1.195, scaleNoise: 2 }
);
