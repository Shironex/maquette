/**
 * The Avatar view in mock mode: each character's VRM spec, a reweight and a
 * normalise that "run" for a moment and make the bundled sheets, the yes or
 * no to each, and the `.vrm` a build writes. `?avatar=` in the URL:
 * - `reweight`: an agent's build moved Elsheria's cape and hair off the arms,
 *   and the new weights wait for a yes (an approval card says so too).
 * - `pending`: the weights were approved, and an agent's build normalised
 *   Elsheria's rest pose, which waits for a yes.
 * - `written`: the pose was approved and `exports/elsheria.vrm` written,
 *   with its report; the Preview tab loads the box humanoid fixture as it.
 * - `round-trip`: like `written`, but the written file did not read back.
 * - `older`: like `pending`, but Elsheria has a newer v3 and the wait is on v2.
 * - `springs`, `chains`: see create-avatar-springs.ts.
 * - `custom`: Elsheria's rig names its joints its own way and has two joints for
 *   each upper leg, which a person assigns in the Bone map (custom-rig-fixtures.ts).
 * Without it Elsheria faces +X in an A-pose and was never built; bone
 * overrides in a saved spec change the bone map as the reader would. */
import type {
  Approval,
  AvatarBuild,
  AvatarExport,
  AvatarSpec_Serialize,
  AvatarView_Serialize,
  Elbows,
  Fingers,
  Normalized,
  ProjectManifest,
  ReweightMode,
  Reweighted,
  VrmReadiness,
  WorkspaceConfig,
} from '../client';
import { metaProblem, specFromDefaults } from '@/state/avatar';
import { clock } from '@/lib/clock';
import { expand, fail, type Args } from './backend-options';
import { DEFAULT_ROOT } from './fixtures';
import { avatarCacheDir } from './avatar-files';
import {
  normalisedReadiness,
  posedReadiness,
  report,
  freeArms,
  roundTripFailed,
  writtenChecks,
} from './avatar-fixtures';
import { customRigReadiness } from './custom-rig-fixtures';
import { CLAY_FIGURE_VRM, withOverrides } from './vrm-fixtures';
import { SPRINGS_SCENARIOS, createSpringsMock } from './create-avatar-springs';

/** Rest pose findings a normalise fixes, as `needs_normalize` reads them. */
const POSE_IDS = new Set(['facing', 'arms', 'fingers', 'ground', 'centre']);

function readinessOf(
  asset: string,
  spec: AvatarSpec_Serialize,
  state: string | null
): VrmReadiness | null {
  const bones = spec.bones;
  if (asset === 'elsheria' && state === 'custom') return customRigReadiness(bones);
  if (asset === 'elsheria') return withOverrides(posedReadiness(), bones);
  if (asset === 'clay-figure') return withOverrides(CLAY_FIGURE_VRM, bones);
  return null;
}

interface Entry {
  spec: AvatarSpec_Serialize | null;
  reweighted: Reweighted | null;
  normalized: Normalized | null;
  export: AvatarExport | null;
  /** An agent's build waits on the new weights. */
  askedReweight: boolean;
  /** An agent's build waits on the normalised pose. */
  asked: boolean;
}

/**
 * `pace` 0 (tests) answers a build at once; otherwise it takes a moment,
 * like Blender would.
 */
export function createAvatarHandlers(
  project: (id: string) => ProjectManifest,
  config: () => WorkspaceConfig,
  state: string | null,
  pace: number
) {
  const root = () => expand(DEFAULT_ROOT);
  const buildMs = pace === 0 ? 0 : 1200;
  const entries = new Map<string, Entry>();
  const key = (p: string, a: string) => `${p}/${a}`;
  const entry = (p: string, a: string): Entry => {
    const k = key(p, a);
    if (!entries.has(k))
      entries.set(k, {
        spec: null,
        reweighted: null,
        normalized: null,
        export: null,
        askedReweight: false,
        asked: false,
      });
    return entries.get(k)!;
  };
  const projectDir = (p: string) => `${root()}/projects/${p}`;
  const springs = createSpringsMock(root, state, (p, a) => assetOf(p, a).name);

  function assetOf(p: string, id: string) {
    const found = (project(p).assets ?? []).find(a => a.id === id || a.name === id);
    if (!found) fail('not-found', `Could not find the asset ${id}.`);
    return found;
  }

  function normalise(
    p: string,
    a: string,
    version: number,
    elbows: Elbows,
    freed: Reweighted | null,
    fingers: Fingers = 'straighten'
  ): Normalized {
    const dir = avatarCacheDir(root(), p, a);
    return {
      project: p,
      asset: a,
      sourceVersion: version,
      state: 'pending',
      glb: `${dir}/normalized.glb`,
      sheets: { before: `${dir}/sheet_before.webp`, after: `${dir}/sheet_after.webp` },
      report: report(elbows, fingers),
      readiness: normalisedReadiness(freed !== null, fingers),
      key: `v${version}-elbows-${elbows}-${fingers}${freed ? '-freed' : ''}`,
      approvedAt: null,
      reweight: freed?.report ?? null,
    };
  }

  function view(p: string, id: string, version: number | null = null): AvatarView_Serialize {
    const asset = assetOf(p, id);
    const latest =
      version === null
        ? (asset.versions.at(-1) ?? null)
        : (asset.versions.find(v => v.version === version) ??
          fail('not-found', `${asset.name} has no version ${version}.`));
    const e = entry(p, asset.id);
    const spec = e.spec ?? specFromDefaults(config().avatar, asset.name);
    const readiness = readinessOf(asset.id, spec, state);
    const normalized =
      e.normalized && e.normalized.sourceVersion === latest?.version ? e.normalized : null;
    const reweighted =
      e.reweighted && e.reweighted.sourceVersion === latest?.version ? e.reweighted : null;
    const findings = readiness?.findings ?? [];
    const needsNormalize = findings.some(f => POSE_IDS.has(f.id));
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
      needsNormalize,
      needsReweight: needsNormalize && findings.some(f => f.id === 'arm-weights'),
      reweighted,
      normalized,
      springs: springs.view(p, asset.id, !needsNormalize || normalized?.state === 'approved'),
      export: e.export,
      exportPath: `${projectDir(p)}/exports/${asset.id}.vrm`,
      metaProblem: metaProblem(spec.meta),
    };
  }

  function write(
    p: string,
    id: string,
    normalized: Normalized | null,
    version: number | null = null
  ): AvatarExport {
    const v = view(p, id, version);
    const unresolved = (normalized?.readiness ?? v.readiness)?.findings ?? [];
    return {
      path: v.exportPath,
      bytes: 9_412_336,
      sourceVersion: v.version ?? 1,
      normalized: normalized !== null,
      reweighted: Boolean(normalized?.reweight),
      material: v.spec.material,
      ...writtenChecks(v.spec),
      unresolved,
      writtenAt: new Date(clock.nowMs()).toISOString(),
      exists: true,
      report: `${v.exportPath}.report.json`,
      springs: springs.written(p, v.asset, v.spec),
    };
  }

  function build(
    p: string,
    id: string,
    version: number | null,
    elbows: Elbows,
    mode: ReweightMode,
    fingers: Fingers
  ): AvatarBuild {
    const v = view(p, id, version);
    if (!v.readiness) fail('invalid', v.readinessError ?? 'No humanoid rig in this model.');
    const problem = metaProblem(v.spec.meta);
    if (problem) fail('invalid', problem);
    const e = entry(p, v.asset);
    const stopped = (): AvatarBuild | null => {
      const hadSprings = (e.export?.springs ?? 0) > 0;
      const { waiting, unpicked } = springs.step(p, v.asset, v.version ?? 1, v.spec, hadSprings);
      if (waiting) return { status: 'springs waiting for approval', chained: waiting, carried: [] };
      if (unpicked) return { status: 'candidates waiting for a pick', found: unpicked };
      return null;
    };
    if (!v.needsNormalize) {
      const stop = stopped();
      if (stop) return stop;
      e.export = write(p, v.asset, null, version);
      return { status: 'written', export: e.export, normalized: null };
    }
    if (v.needsReweight && mode === 'auto') {
      // Another bridges choice is another run, waiting for a yes again.
      const bridges = v.spec.bridges ?? 'hide';
      if (!v.reweighted || v.reweighted.bridges !== bridges) {
        e.reweighted = freeArms(root(), p, v.asset, v.version ?? 1, bridges);
      }
      const reweighted = e.reweighted!;
      if (reweighted.state === 'pending') {
        return { status: 'reweight waiting for approval', reweighted };
      }
    }
    // An approved reweight is what the arms are lifted from.
    const freed = v.reweighted?.state === 'approved' ? v.reweighted : null;
    const wanted = `v${v.version}-elbows-${elbows}-${fingers}${freed ? '-freed' : ''}`;
    if (!e.normalized || e.normalized.key !== wanted) {
      e.normalized = normalise(p, v.asset, v.version ?? 1, elbows, freed, fingers);
    }
    const normalized = e.normalized;
    if (normalized.state === 'pending') return { status: 'waiting for approval', normalized };
    if (normalized.state === 'denied') return { status: 'denied', normalized };
    const stop = stopped();
    if (stop) return stop;
    e.export = write(p, v.asset, normalized, version);
    return { status: 'written', export: e.export, normalized };
  }

  // The scenario's state, as an agent's build left it.
  const written =
    state === 'written' || state === 'round-trip' || SPRINGS_SCENARIOS.has(state ?? '');
  if (state === 'reweight' || state === 'pending' || written || state === 'older') {
    const e = entry('frostveil', 'elsheria');
    e.spec = springs.startSpec({
      ...specFromDefaults(config().avatar, 'Elsheria'),
      meta: { ...specFromDefaults(config().avatar, 'Elsheria').meta, authors: ['The art team'] },
    });
    e.reweighted = freeArms(root(), 'frostveil', 'elsheria', 2);
    e.askedReweight = state === 'reweight';
  }
  if (state === 'pending' || written || state === 'older') {
    const e = entry('frostveil', 'elsheria');
    e.reweighted = {
      ...e.reweighted!,
      state: 'approved',
      approvedAt: new Date(clock.nowMs() - 900_000).toISOString(),
    };
    e.normalized = normalise('frostveil', 'elsheria', 2, 'straighten', e.reweighted);
    e.asked = !written;
    if (written) {
      e.normalized = {
        ...e.normalized,
        state: 'approved',
        approvedAt: new Date(clock.nowMs() - 600_000).toISOString(),
      };
      const done = write('frostveil', 'elsheria', e.normalized);
      e.export = {
        ...done,
        writtenAt: new Date(clock.nowMs() - 540_000).toISOString(),
        ...(state === 'round-trip' ? roundTripFailed(done.unresolved) : {}),
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
    ...springs.handlers,
    avatar_get: ({ project: p, asset, version }) =>
      view(String(p), String(asset), (version as number | null) ?? null),
    avatar_save: ({ project: p, asset, spec }) => {
      const id = assetOf(String(p), String(asset)).id;
      entry(String(p), id).spec = spec as AvatarSpec_Serialize;
      return view(String(p), id);
    },
    avatar_build: ({ project: p, asset, version, elbows, reweight, fingers }) =>
      later(() =>
        build(
          String(p),
          String(asset),
          (version as number | null) ?? null,
          (elbows as Elbows | null) ?? 'straighten',
          (reweight as ReweightMode | null) ?? 'auto',
          (fingers as Fingers | null) ?? 'straighten'
        )
      ),
    avatar_decide: ({ project: p, asset, version, approve }) => {
      const e = entry(String(p), assetOf(String(p), String(asset)).id);
      if (!e.normalized || (version != null && e.normalized.sourceVersion !== version))
        fail('invalid', 'No normalised rest pose waits for an answer.');
      e.normalized = {
        ...e.normalized,
        state: approve ? 'approved' : 'denied',
        approvedAt: approve ? new Date(clock.nowMs()).toISOString() : null,
      };
      e.asked = false;
      return e.normalized;
    },
    avatar_decide_reweight: ({ project: p, asset, version, approve }) => {
      const e = entry(String(p), assetOf(String(p), String(asset)).id);
      if (!e.reweighted || (version != null && e.reweighted.sourceVersion !== version))
        fail('invalid', 'No new weights wait for an answer.');
      e.reweighted = {
        ...e.reweighted,
        state: approve ? 'approved' : 'denied',
        approvedAt: approve ? new Date(clock.nowMs()).toISOString() : null,
      };
      e.askedReweight = false;
      return e.reweighted;
    },
  };

  /** The weights and rest pose approvals agents' builds wait on, for the MCP status. */
  function approvals(): Approval[] {
    const weights = [...entries.values()]
      .filter(e => e.askedReweight && e.reweighted?.state === 'pending')
      .map((e): Approval => {
        const r = e.reweighted!;
        return {
          id: `reweight-${r.asset}`,
          kind: 'reweight',
          project: r.project,
          runId: null,
          runNumber: null,
          step: null,
          job: null,
          assetName: assetOf(r.project, r.asset).name,
          service: null,
          detail: `v${r.sourceVersion}, ${r.report.moved} vertices freed from the arms, ${r.report.keptAsSleeve} kept as sleeve`,
          items: [],
          requestedAtMs: clock.nowMs() - 60_000,
          asset: r.asset,
          version: r.sourceVersion,
          sheets: r.sheets,
        };
      });
    const poses = [...entries.values()]
      .filter(e => e.asked && e.normalized?.state === 'pending')
      .map((e): Approval => {
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
    return [...weights, ...poses, ...springs.approvals()];
  }

  return { handlers, approvals };
}
