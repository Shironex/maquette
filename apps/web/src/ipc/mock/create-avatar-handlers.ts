/**
 * The Avatar view in mock mode: each character asset's VRM spec, a
 * normalise that "runs" for a moment and makes the bundled before and
 * after sheets, the person's yes or no, and the `.vrm` a build writes.
 *
 * `?avatar=` in the URL:
 * - `pending`: an agent's build normalised Elsheria's rest pose, which
 *   waits for a yes (a floating approval card says so too).
 * - `written`: the pose was approved and `exports/elsheria.vrm` written.
 * Without it Elsheria faces +X in an A-pose and was never built.
 */
import type {
  Approval,
  AvatarBuild,
  AvatarExport,
  AvatarSpec_Serialize,
  AvatarView_Serialize,
  Elbows,
  Normalized,
  NormalizeReport,
  ProjectManifest,
  VrmFinding,
  VrmReadiness,
  WorkspaceConfig,
} from '../client';
import { metaProblem, specFromDefaults } from '@/state/avatar';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { CLAY_FIGURE_VRM, vrmReadiness } from './vrm-fixtures';

/** Where the mock's normalise "writes" its sheets, under the cache. */
export function avatarCacheDir(root: string, project: string, asset: string) {
  return `${root}/cache/avatar/${project}/${asset}`;
}

/** The bundled sheet each cached sheet path serves. */
export function avatarSheetFiles(root: string): Record<string, string> {
  const dir = avatarCacheDir(root, 'frostveil', 'elsheria');
  return {
    [`${dir}/sheet_before.webp`]: 'sheet_before.webp',
    [`${dir}/sheet_after.webp`]: 'sheet_after.webp',
  };
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
function posedReadiness(): VrmReadiness {
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
function normalisedReadiness(): VrmReadiness {
  return vrmReadiness(mixamoBones(), 'mixamo', [ARM_WEIGHTS, ...NOTES], {
    groundY: 0,
    hipsOffset: 0.002,
    height: 1.62,
  });
}

function report(elbows: Elbows): NormalizeReport {
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

/** Rest pose findings a normalise fixes, as `needs_normalize` reads them. */
const POSE_IDS = new Set(['facing', 'arms', 'ground', 'centre']);

function readinessOf(asset: string): VrmReadiness | null {
  if (asset === 'elsheria') return posedReadiness();
  if (asset === 'clay-figure') return CLAY_FIGURE_VRM;
  return null;
}

interface Entry {
  spec: AvatarSpec_Serialize | null;
  normalized: Normalized | null;
  export: AvatarExport | null;
  /** An agent's build waits on the normalised pose. */
  asked: boolean;
}

export function createAvatarHandlers({
  project,
  root,
  config,
  state,
  buildMs = 1200,
}: {
  project: (id: string) => ProjectManifest;
  root: () => string;
  config: () => WorkspaceConfig;
  state: string | null;
  buildMs?: number;
}) {
  const entries = new Map<string, Entry>();
  const key = (p: string, a: string) => `${p}/${a}`;
  const entry = (p: string, a: string): Entry => {
    const k = key(p, a);
    if (!entries.has(k))
      entries.set(k, { spec: null, normalized: null, export: null, asked: false });
    return entries.get(k)!;
  };
  const projectDir = (p: string) => `${root()}/projects/${p}`;

  function assetOf(p: string, id: string) {
    const found = (project(p).assets ?? []).find(a => a.id === id || a.name === id);
    if (!found) fail('not-found', `Could not find the asset ${id}.`);
    return found;
  }

  function normalise(p: string, a: string, version: number, elbows: Elbows): Normalized {
    const dir = avatarCacheDir(root(), p, a);
    return {
      project: p,
      asset: a,
      sourceVersion: version,
      state: 'pending',
      glb: `${dir}/normalized.glb`,
      sheets: { before: `${dir}/sheet_before.webp`, after: `${dir}/sheet_after.webp` },
      report: report(elbows),
      readiness: normalisedReadiness(),
      key: `v${version}-elbows-${elbows}`,
      approvedAt: null,
    };
  }

  function view(p: string, id: string): AvatarView_Serialize {
    const asset = assetOf(p, id);
    const latest = asset.versions.at(-1) ?? null;
    const e = entry(p, asset.id);
    const spec = e.spec ?? specFromDefaults(config().avatar, asset.name);
    const readiness = readinessOf(asset.id);
    const normalized =
      e.normalized && e.normalized.sourceVersion === latest?.version ? e.normalized : null;
    return {
      project: p,
      asset: asset.id,
      name: asset.name,
      kind: asset.kind,
      version: latest?.version ?? null,
      model: latest ? `${projectDir(p)}/${latest.file}` : null,
      spec,
      stored: e.spec !== null,
      readiness,
      readinessError: readiness ? null : 'No humanoid rig in this model.',
      needsNormalize: (readiness?.findings ?? []).some(f => POSE_IDS.has(f.id)),
      normalized,
      export: e.export,
      exportPath: `${projectDir(p)}/exports/${asset.id}.vrm`,
      metaProblem: metaProblem(spec.meta),
    };
  }

  function write(p: string, id: string, normalized: Normalized | null): AvatarExport {
    const v = view(p, id);
    const unresolved = (normalized?.readiness ?? v.readiness)?.findings ?? [];
    return {
      path: v.exportPath,
      bytes: 9_412_336,
      sourceVersion: v.version ?? 1,
      normalized: normalized !== null,
      material: v.spec.material,
      unresolved,
      writtenAt: new Date(clock.nowMs()).toISOString(),
      exists: true,
    };
  }

  function build(p: string, id: string, elbows: Elbows): AvatarBuild {
    const v = view(p, id);
    if (!v.readiness) fail('invalid', v.readinessError ?? 'No humanoid rig in this model.');
    const problem = metaProblem(v.spec.meta);
    if (problem) fail('invalid', problem);
    const e = entry(p, v.asset);
    if (!v.needsNormalize) {
      e.export = write(p, v.asset, null);
      return { status: 'written', export: e.export, normalized: null };
    }
    const wanted = `v${v.version}-elbows-${elbows}`;
    if (!e.normalized || e.normalized.key !== wanted) {
      e.normalized = normalise(p, v.asset, v.version ?? 1, elbows);
    }
    const normalized = e.normalized;
    if (normalized.state === 'pending') return { status: 'waiting for approval', normalized };
    if (normalized.state === 'denied') return { status: 'denied', normalized };
    e.export = write(p, v.asset, normalized);
    return { status: 'written', export: e.export, normalized };
  }

  // The scenario's state, as an agent's build left it.
  if (state === 'pending' || state === 'written') {
    const e = entry('frostveil', 'elsheria');
    e.spec = {
      ...specFromDefaults(config().avatar, 'Elsheria'),
      meta: { ...specFromDefaults(config().avatar, 'Elsheria').meta, authors: ['The art team'] },
    };
    e.normalized = normalise('frostveil', 'elsheria', 2, 'straighten');
    e.asked = state === 'pending';
    if (state === 'written') {
      e.normalized = {
        ...e.normalized,
        state: 'approved',
        approvedAt: new Date(clock.nowMs() - 600_000).toISOString(),
      };
      e.export = {
        ...write('frostveil', 'elsheria', e.normalized),
        writtenAt: new Date(clock.nowMs() - 540_000).toISOString(),
      };
    }
  }

  const later = <T>(value: () => T) =>
    buildMs === 0
      ? Promise.resolve().then(value)
      : new Promise<T>((resolve, reject) =>
          setTimeout(() => {
            try {
              resolve(value());
            } catch (error) {
              reject(error);
            }
          }, buildMs)
        );

  const handlers: Record<string, (args: Args) => unknown> = {
    avatar_get: ({ project: p, asset }) => view(String(p), String(asset)),
    avatar_save: ({ project: p, asset, spec }) => {
      const id = assetOf(String(p), String(asset)).id;
      entry(String(p), id).spec = spec as AvatarSpec_Serialize;
      return view(String(p), id);
    },
    avatar_build: ({ project: p, asset, elbows }) =>
      later(() => build(String(p), String(asset), (elbows as Elbows | null) ?? 'straighten')),
    avatar_decide: ({ project: p, asset, approve }) => {
      const e = entry(String(p), assetOf(String(p), String(asset)).id);
      if (!e.normalized) fail('invalid', 'No normalised rest pose waits for an answer.');
      e.normalized = {
        ...e.normalized,
        state: approve ? 'approved' : 'denied',
        approvedAt: approve ? new Date(clock.nowMs()).toISOString() : null,
      };
      e.asked = false;
      return e.normalized;
    },
  };

  /** The rest pose approvals agents' builds wait on, for the MCP status. */
  function approvals(): Approval[] {
    return [...entries.values()]
      .filter(e => e.asked && e.normalized?.state === 'pending')
      .map(e => {
        const n = e.normalized!;
        return {
          id: `rest-pose-${n.asset}`,
          kind: 'rest-pose',
          project: n.project,
          runId: null,
          runNumber: null,
          step: null,
          job: null,
          assetName: assetOf(n.project, n.asset).name,
          service: null,
          detail: `v${n.sourceVersion}, turned 90 degrees, lifted left arm 55 and right arm 53 degrees`,
          items: [],
          requestedAtMs: clock.nowMs() - 90_000,
          asset: n.asset,
          version: n.sourceVersion,
          sheets: n.sheets,
        };
      });
  }

  return { handlers, approvals };
}
