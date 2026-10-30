/**
 * The Avatar view's springs in mock mode: the finder's candidates on the
 * model a VRM is written from, the chains a build adds for the confirmed
 * ones, the person's yes or no to them, and the springs the `.vrm` then
 * carries. Its sheets are the box humanoid fixture's, with a strip hanging
 * from its head.
 *
 * `?avatar=` in the URL:
 * - `springs`: like `written`, and a build looked for dangling parts and
 *   found the strip; nothing is confirmed yet.
 * - `chains`: the strip is confirmed, and an agent's build added its chain,
 *   which waits for a yes (a floating approval card says so too).
 */
import type {
  Approval,
  AvatarSpec_Serialize,
  Chained,
  FoundSprings,
  SpringCollider,
  SpringsView,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { avatarCacheDir } from './avatar-files';

/** The scenarios that start with springs. */
export const SPRINGS_SCENARIOS = new Set(['springs', 'chains']);

/** A capsule collider down (or along) a bone, as the chain step measures them. */
function capsule(group: string, bone: string, joint: string, radius: number, tail: Vec3) {
  return { group, bone, joint, shape: { capsule: { offset: [0, 0, 0] as Vec3, radius, tail } } };
}
type Vec3 = [number, number, number];

/** The box humanoid's colliders: the head, neck, chest, hips, arms and legs. */
const COLLIDERS: SpringCollider[] = [
  {
    group: 'Head',
    bone: 'head',
    joint: 'mixamorig:Head',
    shape: { sphere: { offset: [0, 0.088, 0], radius: 0.0325 } },
  },
  capsule('Neck', 'neck', 'mixamorig:Neck', 0.088, [0, 0.056, 0]),
  capsule('Chest', 'upperChest', 'mixamorig:Spine2', 0.1408, [0, 0.0696, 0]),
  capsule('Chest', 'chest', 'mixamorig:Spine1', 0.1408, [0, 0.0496, 0]),
  capsule('Hips', 'hips', 'mixamorig:Hips', 0.1408, [0, 0.0296, 0]),
  capsule('Arms', 'leftUpperArm', 'mixamorig:LeftArm', 0.0704, [0.2248, 0, 0]),
  capsule('Arms', 'rightUpperArm', 'mixamorig:RightArm', 0.0704, [-0.2248, 0, 0]),
  capsule('Legs', 'leftUpperLeg', 'mixamorig:LeftUpLeg', 0.0704, [0, -0.3848, 0]),
  capsule('Legs', 'rightUpperLeg', 'mixamorig:RightUpLeg', 0.0704, [0, -0.3848, 0]),
];

/** The strip down the box humanoid's back, as the finder proposes it. */
function found(dir: string): FoundSprings {
  return {
    candidates: [
      {
        id: 'hair-1',
        kind: 'hair',
        rootBone: 'head',
        line: [
          [0, 1.62, -0.3],
          [0, 1.31, -0.3],
          [0, 1.0, -0.3],
        ],
        length: 0.62,
        drop: 0.62,
        vertices: 792,
        confidence: 0.99,
        joints: 5,
        color: '#e8553f',
      },
    ],
    sheet: `${dir}/find/candidates.webp`,
    input: `${dir}/normalized.glb`,
    height: 1.76,
  };
}

function chain(p: string, a: string, version: number, ids: string[]): Chained {
  const dir = `${avatarCacheDir('', p, a)}/springs/chain`;
  const joints = (id: string) => Array.from({ length: 5 }, (_, k) => `spring_${id}_${k}`);
  return {
    project: p,
    asset: a,
    sourceVersion: version,
    state: 'pending',
    glb: `${dir}/springs.glb`,
    sheets: { before: `${dir}/sheet_before.webp`, after: `${dir}/sheet_after.webp` },
    report: {
      height: 1.76,
      chains: ids.map(id => ({
        id,
        kind: 'hair',
        rootBone: 'head',
        rootJoint: 'mixamorig:Head',
        joints: joints(id),
        length: 0.62,
        vertices: 792,
        blended: 48,
        heads: [],
        radius: 0.012,
        colliderGroups: ['Head', 'Neck', 'Chest', 'Arms', 'Hips', 'Legs'],
      })),
      colliders: COLLIDERS,
      weights: { vertices: 2376, maxInfluences: 3, unweighted: 0, offSum: 0 },
      changed: 792,
    },
    key: `v${version}-${ids.join('-')}`,
    approvedAt: null,
  };
}

export function createSpringsMock(
  root: () => string,
  state: string | null,
  name: (p: string, a: string) => string
) {
  const looks = new Map<string, FoundSprings>();
  const chains = new Map<string, Chained>();
  const asked = new Set<string>();
  const key = (p: string, a: string) => `${p}/${a}`;
  const dir = (p: string, a: string) => `${avatarCacheDir(root(), p, a)}/springs`;
  const rooted = (c: Chained): Chained => ({
    ...c,
    glb: `${root()}${c.glb}`,
    sheets: { before: `${root()}${c.sheets.before}`, after: `${root()}${c.sheets.after}` },
  });

  if (state && SPRINGS_SCENARIOS.has(state)) {
    looks.set(key('frostveil', 'elsheria'), found(dir('frostveil', 'elsheria')));
  }
  if (state === 'chains') {
    chains.set(key('frostveil', 'elsheria'), rooted(chain('frostveil', 'elsheria', 2, ['hair-1'])));
    asked.add(key('frostveil', 'elsheria'));
  }

  /** The spec a scenario starts with: the strip confirmed in `chains`. */
  function startSpec(spec: AvatarSpec_Serialize): AvatarSpec_Serialize {
    if (state !== 'chains') return spec;
    const strip = found('').candidates[0]!;
    return {
      ...spec,
      springs: [{ id: strip.id, kind: strip.kind, rootBone: strip.rootBone, line: strip.line }],
    };
  }

  function view(p: string, a: string, ready: boolean): SpringsView {
    if (!ready) return { ready: false, found: null, chained: null };
    return {
      ready,
      found: looks.get(key(p, a)) ?? null,
      chained: chains.get(key(p, a)) ?? null,
    };
  }

  /**
   * What a build does about springs: looks once with none confirmed, and
   * with some, makes their chains (again when they changed) and waits on
   * them while they are pending.
   */
  function step(p: string, a: string, version: number, spec: AvatarSpec_Serialize) {
    const ids = (spec.springs ?? []).map(s => s.id);
    if (ids.length === 0) {
      if (!looks.has(key(p, a))) looks.set(key(p, a), found(dir(p, a)));
      return { waiting: null };
    }
    const made = chains.get(key(p, a));
    const wanted = `v${version}-${ids.join('-')}`;
    if (!made || made.key !== wanted) {
      chains.set(key(p, a), rooted(chain(p, a, version, ids)));
    }
    const current = chains.get(key(p, a))!;
    return { waiting: current.state === 'pending' ? current : null };
  }

  /** The springs a file written now carries: the confirmed ones, once their chains are approved. */
  function written(p: string, a: string, spec: AvatarSpec_Serialize): number {
    const made = chains.get(key(p, a));
    return made?.state === 'approved' ? (spec.springs ?? []).length : 0;
  }

  const handlers: Record<string, (args: Args) => unknown> = {
    avatar_find_springs: ({ project: p, asset }) => {
      const k = key(String(p), String(asset));
      if (!looks.has(k)) looks.set(k, found(dir(String(p), String(asset))));
      return looks.get(k);
    },
    avatar_decide_springs: ({ project: p, asset, approve }) => {
      const k = key(String(p), String(asset));
      const made = chains.get(k);
      if (!made) fail('invalid', 'No spring chains wait for an answer.');
      const decided: Chained = {
        ...made,
        state: approve ? 'approved' : 'denied',
        approvedAt: approve ? new Date(clock.nowMs()).toISOString() : null,
      };
      chains.set(k, decided);
      asked.delete(k);
      return decided;
    },
  };

  /** The spring chains agents' builds wait on, for the MCP status. */
  function approvals(): Approval[] {
    return [...chains.entries()]
      .filter(([k, c]) => asked.has(k) && c.state === 'pending')
      .map(([, c]) => ({
        id: `springs-${c.asset}`,
        kind: 'springs',
        project: c.project,
        runId: null,
        runNumber: null,
        step: null,
        job: null,
        assetName: name(c.project, c.asset),
        service: null,
        detail: `v${c.sourceVersion}, 1 spring chain (hair-1), 792 vertices reweighted`,
        items: [],
        requestedAtMs: clock.nowMs() - 45_000,
        asset: c.asset,
        version: c.sourceVersion,
        sheets: c.sheets,
      }));
  }

  return { view, step, written, startSpec, handlers, approvals };
}
