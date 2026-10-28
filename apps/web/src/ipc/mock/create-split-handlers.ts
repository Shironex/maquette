/**
 * Splits on a provider and assembled props in mock mode. A split goes
 * through the fake provider queue like a send and lands in the Inbox; a
 * bake back adds the next version at once; an assembly adds a version of
 * the prop with a node per part.
 *
 * `?parts=` gives Frostveil the three Sigil gate parts (frame, door leaf,
 * steps) as models of their own, for Assemble. `?parts=split` adds a Tripo
 * split of the frost lantern, accepted as v3, that lost its texture and
 * offers the bake back from v2.
 */
import type {
  Asset,
  AssembleRequest,
  Assembled,
  KeyStatus,
  ProjectManifest,
  ProviderId,
  ProviderJob,
  Recipe,
  SplitRequest,
  SplitSheet,
  SplitSource,
} from '../client';
import { clock } from '@/lib/clock';
import { DEFAULT_TRIPO, assembleProblem, meshyProblem, splitCredits } from '@/state/split';
import { fail, type Args } from './backend-options';
import type { createFakeProviders } from './providers';

type Providers = ReturnType<typeof createFakeProviders>;

const PRICES = { tripoSegment: 40, tripoComplete: 100, meshySplit: 10 };
const TAILS: Record<ProviderId, string> = { tripo: '4f2a', meshy: '9c1e' };

/** What made each sample model, and its numbers. */
const MADE: Record<string, Partial<SplitSource>> = {
  'frost-lantern': { madeBy: 'tripo', tripoTask: 'tripo-task-lantern', triangles: 18_200 },
  'rune-chest': {
    madeBy: 'meshy',
    tripoTask: null,
    meshyProblem: null,
    triangles: 8_600,
    materials: 2,
  },
};

const GATE = [
  { id: 'sigil-gate-frame', name: 'Sigil gate frame', triangles: 21_400, made: 'tripo' },
  { id: 'sigil-gate-door', name: 'Sigil gate door', triangles: 6_200, made: 'tripo' },
  { id: 'sigil-gate-steps', name: 'Sigil gate steps', triangles: 4_800, made: 'meshy' },
] as const;

/** Project files to the bundled model each one serves, for `?parts=`. */
export const SPLIT_FILES: Record<string, string> = {
  'models/frost_lantern_v3.glb': 'frost_lantern_tripo.glb',
  'models/frost_lantern_v4.glb': 'frost_lantern_split.glb',
  'models/sigil_gate_frame_v1.glb': 'sigil_gate_frame.glb',
  'models/sigil_gate_door_v1.glb': 'sigil_gate_door.glb',
  'models/sigil_gate_steps_v1.glb': 'sigil_gate_steps.glb',
  'models/sigil_gate_v1.glb': 'sigil_gate.glb',
};

const stamp = () => new Date(clock.nowMs() - 3 * 3_600_000).toISOString();

function withVersion(asset: Asset, triangles: number, textures: number): Asset {
  const n = (asset.versions.at(-1)?.version ?? 0) + 1;
  const stem = asset.id.replace(/-/g, '_');
  const version = {
    version: n,
    file: `models/${stem}_v${n}.glb`,
    createdAt: new Date(clock.nowMs()).toISOString(),
    stats: { triangles, bytes: 980_000, textures, maxTexture: 1024, joints: 0, animations: 0 },
  };
  return { ...asset, versions: [...asset.versions, version], updatedAt: version.createdAt };
}

function gateAsset(id: string, name: string, triangles: number): Asset {
  const at = stamp();
  const empty: Asset = {
    id,
    name,
    kind: 'prop',
    stage: 'blender',
    status: 'review',
    versions: [],
    thumbnail: null,
    createdAt: at,
    updatedAt: at,
  };
  return withVersion(empty, triangles, 1);
}

interface SplitDeps {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  providers: Providers;
  scenario: string | null;
}

export function createSplitHandlers({ project, saveProject, providers, scenario }: SplitDeps) {
  const recipes = new Map<string, Recipe>();
  // The demo has both keys, so each way of splitting can be tried.
  const keys = (): KeyStatus[] =>
    (['tripo', 'meshy'] as const).map(service => ({
      service,
      saved: Boolean(scenario) || providers.hasKey(service),
      tail: scenario || providers.hasKey(service) ? TAILS[service] : null,
    }));
  const assetOf = (p: ProjectManifest, id: string): Asset =>
    (p.assets ?? []).find(a => a.id === id || a.name === id) ??
    fail('not-found', `Could not find the asset ${id}.`);
  const replace = (p: ProjectManifest, asset: Asset) =>
    saveProject({ ...p, assets: (p.assets ?? []).map(a => (a.id === asset.id ? asset : a)) });

  if (scenario) seed();

  function seed() {
    const p = project('frostveil');
    const gate = GATE.map(g => gateAsset(g.id, g.name, g.triangles));
    const assets = (p.assets ?? []).map(a =>
      a.id === 'frost-lantern' && scenario === 'split' ? withVersion(a, 18_200, 0) : a
    );
    saveProject({ ...p, assets: [...assets, ...gate] });
    if (scenario !== 'split') return;
    const job = providers.base({
      id: 'frost-lantern-split-1',
      asset: 'frost-lantern',
      assetName: 'Frost lantern',
      provider: 'tripo',
      tripo: null,
      state: 'accepted',
      version: 3,
      credits: 40,
      estimate: 40,
      cancel: 'none',
      createdAtMs: clock.nowMs() - 40 * 60_000,
      finishedAtMs: clock.nowMs() - 30 * 60_000,
      split: {
        fromVersion: 2,
        fromItem: null,
        tripo: {
          model: 'semantic',
          granularity: 'balanced',
          splitByConnectivity: true,
          complete: null,
        },
        meshy: null,
        inputTask: 'tripo-task-lantern',
        texturesBefore: 1,
        textureKept: false,
        bake: { state: 'offered', onto: 3, version: null, message: null },
      },
    });
    providers.seed(job);
  }

  function source(req: Pick<SplitRequest, 'project' | 'asset' | 'version' | 'item'>): SplitSource {
    const asset = assetOf(project(req.project), req.asset);
    const version = req.item ? null : (req.version ?? asset.versions.at(-1)?.version ?? null);
    const stats = asset.versions.find(v => v.version === version)?.stats;
    const label = version !== null ? `v${version}` : 'the Inbox model';
    const gate = GATE.find(g => g.id === asset.id);
    const made = MADE[asset.id] ?? { madeBy: gate?.made ?? null };
    const fromMeshy = made.madeBy === 'meshy';
    const busy = providers
      .jobs()
      .find(
        j => j.asset === asset.id && j.split && ['approval', 'queued', 'running'].includes(j.state)
      );
    return {
      project: req.project,
      asset: asset.id,
      assetName: asset.name,
      version,
      item: req.item ?? null,
      madeBy: null,
      tripoTask: null,
      meshyProblem: fromMeshy
        ? null
        : `Meshy splits only models it made, and ${label} did not come from Meshy.`,
      triangles: stats?.triangles ?? 12_200,
      materials: 4,
      textures: stats?.textures ?? 1,
      objects: 1,
      busy: busy?.id ?? null,
      ...made,
    };
  }

  return {
    split_sheet: ({ project: p, asset, version, item }: Args): SplitSheet => ({
      source: source({
        project: String(p),
        asset: String(asset),
        version: (version as number | null) ?? null,
        item: (item as string | null) ?? null,
      }),
      keys: keys(),
      prices: PRICES,
    }),
    split_send: ({ request }: Args): ProviderJob => {
      const req = request as SplitRequest;
      const from = source(req);
      const service = req.provider === 'meshy' ? 'meshy' : 'tripo';
      if (!keys().find(k => k.service === service)?.saved) {
        fail('invalid', `Save a ${service === 'tripo' ? 'Tripo' : 'Meshy'} API key first.`);
      }
      if (from.busy) fail('invalid', `A split of ${from.assetName} is on its way already.`);
      const problem = service === 'meshy' ? (from.meshyProblem ?? meshyProblem(req.meshy!)) : null;
      if (problem) fail('invalid', problem);
      const tripo = req.tripo ?? null;
      const credits = splitCredits(service, tripo ?? DEFAULT_TRIPO, PRICES);
      const used = providers.jobs().filter(j => j.asset === from.asset).length;
      const job = providers.base({
        id: `${from.asset}-${used + 1}`,
        project: req.project,
        asset: from.asset,
        assetName: from.assetName,
        provider: service,
        service,
        tripo: null,
        meshy: null,
        estimate: credits,
        createdAtMs: clock.nowMs(),
        split: {
          fromVersion: from.version,
          fromItem: from.item,
          tripo,
          meshy: req.meshy ?? null,
          inputTask: service === 'tripo' ? from.tripoTask : 'meshy-task',
          texturesBefore: from.textures,
          textureKept: null,
          bake: null,
        },
      });
      return providers.start(job);
    },
    split_bake_back: ({ id }: Args): ProviderJob => {
      const job =
        providers.jobs().find(j => j.id === String(id)) ??
        fail('not-found', `Could not find the job ${String(id)}.`);
      const bake = job.split?.bake ?? fail('invalid', 'This split has nothing to bake back.');
      if (bake.state === 'done' || bake.state === 'running') {
        fail('invalid', 'The texture is already back, or baking.');
      }
      const p = project(job.project);
      const asset = withVersion(assetOf(p, job.asset), 18_200, 1);
      providers.changed(replace(p, asset));
      const made = asset.versions.at(-1)?.version ?? null;
      const from = job.split?.fromVersion ? `v${job.split.fromVersion}` : 'the Inbox model';
      const message = `Texture baked back from ${from} as v${made}. v${bake.onto} keeps the untextured parts.`;
      return providers.put({
        ...job,
        split: { ...job.split!, bake: { ...bake, state: 'done', version: made, message } },
      });
    },
    assemble_prop: ({ request }: Args): Assembled => {
      const req = request as AssembleRequest;
      const p = project(req.project);
      const drafts = req.parts.map(part => ({ ...part, triangles: 0 }));
      const problem = assembleProblem(req.name, drafts);
      if (problem) fail('invalid', problem);
      const name = req.name.trim();
      const triangles = req.parts.reduce(
        (sum, part) => sum + (assetOf(p, part.asset).versions.at(-1)?.stats?.triangles ?? 0),
        0
      );
      const id = req.asset ?? name.replace(/_/g, '-');
      const existing = (p.assets ?? []).find(a => a.id === id);
      const prop = withVersion(
        existing ?? { ...gateAsset(id, name, 0), versions: [] },
        triangles,
        1
      );
      const next = saveProject({
        ...p,
        assets: existing
          ? (p.assets ?? []).map(a => (a.id === id ? prop : a))
          : [prop, ...(p.assets ?? [])],
      });
      recipes.set(`${p.id}/${id}`, { name, parts: req.parts });
      const version = prop.versions.at(-1)!;
      return {
        project: next,
        asset: id,
        version,
        root: name,
        nodes: req.parts.map(part => ({ name: part.name.trim(), at: part.at ?? [0, 0, 0] })),
      };
    },
    assemble_recipe: ({ project: p, asset }: Args): Recipe | null =>
      recipes.get(`${String(p)}/${String(asset)}`) ?? null,
  } as Record<string, (args: Args) => unknown>;
}
