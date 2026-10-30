/** Fixtures for the mock Avatar view: a posed rig, its normalised twin, and the older-version seed. */
import type { Elbows, NormalizeReport, ProjectManifest, VrmFinding, VrmReadiness } from '../client';
import { vrmReadiness } from './vrm-fixtures';

/** `?avatar=older`: Elsheria gets a v3 over the v2 an agent's pose waits on. */
export function seedAvatar(projects: ProjectManifest[], avatar: string | null): ProjectManifest[] {
  if (avatar !== 'older') return projects;
  return projects.map(p => ({
    ...p,
    assets: p.assets?.map(a => {
      const last = a.versions.at(-1);
      if (a.id !== 'elsheria' || !last) return a;
      const next = {
        ...last,
        version: last.version + 1,
        file: last.file.replace(/_v\d+/, `_v${last.version + 1}`),
      };
      return { ...a, versions: [...a.versions, next] };
    }),
  }));
}

const SIDES = ['Left', 'Right'] as const;

/** Mixamo joint names for every bone but the eyes and the jaw. */
function mixamoBones(): Record<string, string> {
  const map: Record<string, string> = {
    hips: 'Hips',
    spine: 'Spine',
    chest: 'Spine1',
    upperChest: 'Spine2',
    neck: 'Neck',
    head: 'Head',
  };
  for (const side of SIDES) {
    const s = side.toLowerCase();
    Object.assign(map, {
      [`${s}UpperLeg`]: `${side}UpLeg`,
      [`${s}LowerLeg`]: `${side}Leg`,
      [`${s}Foot`]: `${side}Foot`,
      [`${s}Toes`]: `${side}ToeBase`,
      [`${s}Shoulder`]: `${side}Shoulder`,
      [`${s}UpperArm`]: `${side}Arm`,
      [`${s}LowerArm`]: `${side}ForeArm`,
      [`${s}Hand`]: `${side}Hand`,
      [`${s}ThumbMetacarpal`]: `${side}HandThumb1`,
      [`${s}ThumbProximal`]: `${side}HandThumb2`,
      [`${s}ThumbDistal`]: `${side}HandThumb3`,
    });
    for (const finger of ['Index', 'Middle', 'Ring', 'Little']) {
      const joint = finger === 'Little' ? 'Pinky' : finger;
      map[`${s}${finger}Proximal`] = `${side}Hand${joint}1`;
      map[`${s}${finger}Intermediate`] = `${side}Hand${joint}2`;
      map[`${s}${finger}Distal`] = `${side}Hand${joint}3`;
    }
  }
  return Object.fromEntries(Object.entries(map).map(([bone, j]) => [bone, `mixamorig:${j}`]));
}

const ARM_WEIGHTS: VrmFinding = {
  id: 'arm-weights',
  level: 'warning',
  title: '212 vertices of cloth or hair weighted to the arms',
  detail:
    'They move with leftUpperArm (118), rightUpperArm (86) and leftLowerArm (8) but sit more ' +
    'than 13 cm from the arm, like a cape or hair beside it. Lifting the arms to a T-pose drags ' +
    'them along, so check the shoulders on the after sheet. A reweight that frees them is planned.',
};

const NOTES: VrmFinding[] = [
  {
    id: 'bones',
    level: 'note',
    title: '52 of 55 humanoid bones mapped',
    detail: '30 finger bones among them. Not found, all optional: leftEye, rightEye and jaw.',
  },
  {
    id: 'animations',
    level: 'note',
    title: '2 animations are dropped on export',
    detail: 'VRM carries no clips. Motion ships as VRMA files instead.',
  },
];

/** A rigged character as Tripo hands it over: facing +X, arms 55 degrees down. */
export function posedReadiness(): VrmReadiness {
  return vrmReadiness(
    mixamoBones(),
    'mixamo',
    [
      {
        id: 'facing',
        level: 'blocker',
        title: 'Faces +X, VRM wants +Z',
        detail:
          'The front is turned 90 degrees from +Z. Turning it is part of normalising the rest pose.',
      },
      {
        id: 'arms',
        level: 'blocker',
        title: 'Arms hang below horizontal, VRM wants a T-pose',
        detail:
          'Upper arm to hand: left 55 degrees below and right 53 degrees below. Normalising the ' +
          'rest pose lifts them.',
      },
      ARM_WEIGHTS,
      ...NOTES,
    ],
    {
      facing: '+x',
      facingDeg: 90,
      armLeftDeg: 55,
      armRightDeg: 53,
      groundY: 0.004,
      hipsOffset: 0.012,
      height: 1.62,
    }
  );
}

/** The same character after the normalise: only the arm weights are left. */
export function normalisedReadiness(): VrmReadiness {
  return vrmReadiness(mixamoBones(), 'mixamo', [ARM_WEIGHTS, ...NOTES], {
    groundY: 0,
    hipsOffset: 0.002,
    height: 1.62,
  });
}

export function report(elbows: Elbows): NormalizeReport {
  const kept = elbows === 'keep';
  const before = {
    facingDeg: 90,
    armLeftDeg: 55,
    armRightDeg: 53,
    elbowLeftDeg: 14,
    elbowRightDeg: 12,
    groundY: 0.004,
    hipsOffset: 0.012,
    height: 1.62,
    vertices: 48_211,
  };
  return {
    before,
    after: {
      ...before,
      facingDeg: 0,
      armLeftDeg: 0.4,
      armRightDeg: 0.3,
      elbowLeftDeg: kept ? 14 : 0.2,
      elbowRightDeg: kept ? 12 : 0.1,
      groundY: 0,
      hipsOffset: 0.002,
    },
    turnedDeg: 90,
    moved: [0, -0.004, 0],
    elbows,
    lifted: {
      left: { shoulderDeg: 55, elbowDeg: kept ? null : 14 },
      right: { shoulderDeg: 53, elbowDeg: kept ? null : 12 },
    },
  };
}
