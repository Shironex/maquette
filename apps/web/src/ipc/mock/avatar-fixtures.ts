/**
 * Fixtures for the mock Avatar view: a posed rig, its normalised twin, what
 * freeing its arms moved, a round trip that failed, the highlights a written
 * MToon file softened, and the older-version seed.
 */
import type {
  AvatarExport,
  AvatarSpec,
  BridgeFaces,
  Elbows,
  Fingers,
  NormalizeReport,
  ProjectManifest,
  ReweightReport,
  Reweighted,
  VrmFinding,
  VrmReadiness,
} from '../client';
import { avatarCacheDir } from './avatar-files';
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
    "than twice the arm's own radius from it, like a cape or hair beside it. Lifting the arms " +
    'to a T-pose drags them along. Free them first: a reweight keeps the arms, sleeves and ' +
    'gloves and moves the rest to the trunk, for a person to approve on before and after sheets.',
};

/** What the reweight left on the arms, once a person approved it. */
const ARM_WEIGHTS_LEFT: VrmFinding = {
  id: 'arm-weights',
  level: 'note',
  title: '64 vertices still follow the arms after the reweight, cloth sewn to the sleeves',
  detail:
    'The reweight freed the arms and kept these on leftUpperArm (40) and rightUpperArm (24) ' +
    'as sleeve: they sit beside the arm but join it, so they stretch with it instead of tearing.',
};

const FINGERS_CURLED: VrmFinding = {
  id: 'fingers',
  level: 'warning',
  title: 'Fingers curl up to 64 degrees, VRM wants them straight',
  detail:
    "Most at leftRing (64), rightRing (62) and leftLittle (58). VRM's rest pose lays the four " +
    'fingers straight along the hand and the thumb 45 degrees between the hand and the front, ' +
    "and animations retarget onto that. Normalising the rest pose straightens them as far as the hand's " +
    'mesh holds; fused fingers keep some curl.',
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
      FINGERS_CURLED,
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
      fingerCurlDeg: 64,
    }
  );
}

/**
 * The same character after the normalise: only the arm weights are left,
 * as what the reweight kept when it was `freed` first, and the fingers
 * when they were kept.
 */
export function normalisedReadiness(freed = false, fingers: Fingers = 'straighten'): VrmReadiness {
  const left = [
    ...(fingers === 'keep' ? [FINGERS_CURLED] : []),
    freed ? ARM_WEIGHTS_LEFT : ARM_WEIGHTS,
  ];
  return vrmReadiness(mixamoBones(), 'mixamo', [...left, ...NOTES], {
    groundY: 0,
    hipsOffset: 0.002,
    height: 1.62,
    fingerCurlDeg: fingers === 'keep' ? 64 : 2,
  });
}

export function report(elbows: Elbows, fingers: Fingers = 'straighten'): NormalizeReport {
  const kept = elbows === 'keep';
  const straight = fingers === 'straighten';
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
    fingerCurlDeg: 64,
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
      fingerCurlDeg: straight ? 2 : 64,
    },
    turnedDeg: 90,
    moved: [0, -0.004, 0],
    elbows,
    lifted: {
      left: { shoulderDeg: 55, elbowDeg: kept ? null : 14 },
      right: { shoulderDeg: 53, elbowDeg: kept ? null : 12 },
    },
    fingers,
    straightened: straight
      ? {
          left: { share: 1, stretch: 1.21, turnedDeg: 42 },
          right: { share: 0.75, stretch: 1.31, turnedDeg: 33 },
        }
      : {},
  };
}

/**
 * What freeing the arms moved: the cape and hair off them, the sleeves kept,
 * and a strap fused from the right cuff to the hip left as a bridge, hidden
 * unless the spec keeps it.
 */
export function reweightReport(bridges: BridgeFaces = 'hide'): ReweightReport {
  return {
    vertices: 48_211,
    armWeighted: 4_745,
    moved: 2_314,
    weightMoved: 1_125.6,
    keptAsSleeve: 2_410,
    band: 1_072,
    fromBones: {
      'mixamorig:LeftArm': 1_109,
      'mixamorig:RightForeArm': 309,
      'mixamorig:LeftForeArm': 224,
    },
    toBones: {
      'mixamorig:Spine2': 2_201,
      'mixamorig:Spine1': 2_150,
      'mixamorig:LeftShoulder': 412,
    },
    followArm: 640,
    bridgeFaces: 17,
    bridges: [{ side: 'right', faces: 17, at: [0.105, 0.981, 0.08], length: 0.481 }],
    hiddenFaces: bridges === 'hide' ? 17 : 0,
    weights: { vertices: 48_211, maxInfluences: 4, unweighted: 0, offSum: 0 },
    sides: {
      left: {
        moved: 1_602,
        keptAsSleeve: 1_296,
        limbRadius: [0.05, 0.014, 0.012],
        bridgeFaces: 0,
      },
      right: {
        moved: 712,
        keptAsSleeve: 1_114,
        limbRadius: [0.05, 0.015, 0.012],
        bridgeFaces: 17,
      },
    },
  };
}

/** A pending reweight of `version`, with the bridges hidden or kept. */
export function freeArms(
  root: string,
  p: string,
  a: string,
  version: number,
  bridges: BridgeFaces = 'hide'
): Reweighted {
  const dir = `${avatarCacheDir(root, p, a)}/reweight`;
  return {
    project: p,
    asset: a,
    sourceVersion: version,
    state: 'pending',
    glb: `${dir}/reweighted.glb`,
    sheets: { before: `${dir}/sheet_before.webp`, after: `${dir}/sheet_after.webp` },
    report: reweightReport(bridges),
    bridges,
    key: `v${version}-reweight-${bridges}`,
    approvedAt: null,
  };
}

/** `?avatar=round-trip`: the written file did not read back, a blocker in what it lacks. */
export function roundTripFailed(
  unresolved: VrmFinding[]
): Pick<AvatarExport, 'roundTrip' | 'unresolved'> {
  const problems = [
    'extensionsUsed does not list VRMC_materials_mtoon.',
    'leftLowerArm sits under leftShoulder in the node tree; VRM puts it under leftUpperArm.',
  ];
  return {
    roundTrip: { passed: false, problems, bones: 52 },
    unresolved: [
      {
        id: 'round-trip',
        level: 'blocker',
        title: 'The written file does not read back as a VRM',
        detail: problems.join(' '),
      },
      ...unresolved,
    ],
  };
}

/**
 * A written file's checks: it reads back, and with MToon what it did with
 * its baked highlights (3% of the texture softened).
 */
export function writtenChecks(spec: AvatarSpec): Pick<AvatarExport, 'roundTrip' | 'highlights'> {
  const roundTrip = { passed: true, problems: [], bones: 52 };
  if (spec.material !== 'mtoon-soft') return { roundTrip, highlights: null };
  if (spec.highlights === 'keep')
    return { roundTrip, highlights: { mode: 'keep', strength: 0.75, images: [], note: null } };
  return {
    roundTrip,
    highlights: {
      mode: 'soften',
      strength: 0.75,
      images: [
        {
          image: 1,
          width: 2048,
          height: 2048,
          coverage: 0.0298,
          darkCoverage: 0.0322,
          softened: true,
          note: null,
        },
      ],
      note: null,
    },
  };
}
