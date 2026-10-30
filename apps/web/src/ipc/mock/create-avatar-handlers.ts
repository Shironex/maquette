/**
 * The Avatar view in mock mode: each character asset's VRM spec, a
 * normalise that "runs" for a moment and makes the bundled before and
 * after sheets, the person's yes or no, and the `.vrm` a build writes.
 *
 * `?avatar=` in the URL:
 * - `pending`: an agent's build normalised Elsheria's rest pose, which
 *   waits for a yes (a floating approval card says so too).
 * - `written`: the pose was approved and `exports/elsheria.vrm` written.
 * - `older`: like `pending`, but Elsheria has a newer v3 and the wait is on v2.
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
  ProjectManifest,
  VrmReadiness,
  WorkspaceConfig,
} from '../client';
import { metaProblem, specFromDefaults } from '@/state/avatar';
import { clock } from '@/lib/clock';
import { expand, fail, type Args } from './backend-options';
import { DEFAULT_ROOT } from './fixtures';
import { normalisedReadiness, posedReadiness, report } from './avatar-fixtures';
import { CLAY_FIGURE_VRM } from './vrm-fixtures';

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

  function view(p: string, id: string, version: number | null = null): AvatarView_Serialize {
    const asset = assetOf(p, id);
    const latest =
      version === null
        ? (asset.versions.at(-1) ?? null)
        : (asset.versions.find(v => v.version === version) ??
          fail('not-found', `${asset.name} has no version ${version}.`));
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
      material: v.spec.material,
      unresolved,
      writtenAt: new Date(clock.nowMs()).toISOString(),
      exists: true,
    };
  }

  function build(p: string, id: string, version: number | null, elbows: Elbows): AvatarBuild {
    const v = view(p, id, version);
    if (!v.readiness) fail('invalid', v.readinessError ?? 'No humanoid rig in this model.');
    const problem = metaProblem(v.spec.meta);
    if (problem) fail('invalid', problem);
    const e = entry(p, v.asset);
    if (!v.needsNormalize) {
      e.export = write(p, v.asset, null, version);
      return { status: 'written', export: e.export, normalized: null };
    }
    const wanted = `v${v.version}-elbows-${elbows}`;
    if (!e.normalized || e.normalized.key !== wanted) {
      e.normalized = normalise(p, v.asset, v.version ?? 1, elbows);
    }
    const normalized = e.normalized;
    if (normalized.state === 'pending') return { status: 'waiting for approval', normalized };
    if (normalized.state === 'denied') return { status: 'denied', normalized };
    e.export = write(p, v.asset, normalized, version);
    return { status: 'written', export: e.export, normalized };
  }

  // The scenario's state, as an agent's build left it.
  if (state === 'pending' || state === 'written' || state === 'older') {
    const e = entry('frostveil', 'elsheria');
    e.spec = {
      ...specFromDefaults(config().avatar, 'Elsheria'),
      meta: { ...specFromDefaults(config().avatar, 'Elsheria').meta, authors: ['The art team'] },
    };
    e.normalized = normalise('frostveil', 'elsheria', 2, 'straighten');
    e.asked = state !== 'written';
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
    avatar_get: ({ project: p, asset, version }) =>
      view(String(p), String(asset), (version as number | null) ?? null),
    avatar_save: ({ project: p, asset, spec }) => {
      const id = assetOf(String(p), String(asset)).id;
      entry(String(p), id).spec = spec as AvatarSpec_Serialize;
      return view(String(p), id);
    },
    avatar_build: ({ project: p, asset, version, elbows }) =>
      later(() =>
        build(
          String(p),
          String(asset),
          (version as number | null) ?? null,
          (elbows as Elbows | null) ?? 'straighten'
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
