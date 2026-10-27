/**
 * Real sizes and fix-ups in mock mode: each project keeps its `prepare.json`
 * in memory, checked the way the core checks it, and a preview works out
 * the numbers the headless script would report instead of running it.
 */
import type {
  AssetKind,
  AssetPrep,
  Fixup,
  FixupDefaults,
  FixupReport,
  FixupStep,
  InboxItem,
  ModelNumbers,
  ModelPrep,
  PreviewRequest,
  ProjectManifest,
  ScenePrep,
  Suggestion,
  WorkspaceConfig,
} from '../client';
import {
  MAX_METRES,
  MIN_METRES,
  extentOf,
  frameOf,
  framed,
  metres,
  scaleFor,
} from '@/lib/real-size';
import { FIXUP_LABEL, FIXUP_ORDER } from '@/state/prepare';
import { fail, sleep, type Args } from './backend-options';

export { createFakeBake } from './bake';

type Bounds = { min: [number, number, number]; max: [number, number, number] };

/** A few of the core's everyday sizes, for the suggestion. */
const TABLE: [string, Suggestion['size']][] = [
  ['lantern post', { axis: 'height', value: 2.2, unit: 'm' }],
  ['laptop', { axis: 'width', value: 32, unit: 'cm' }],
  ['mug', { axis: 'height', value: 10, unit: 'cm' }],
  ['crystal lantern', { axis: 'height', value: 1.8, unit: 'm' }],
  ['gate', { axis: 'height', value: 6, unit: 'm' }],
  ['crystals', { axis: 'height', value: 1.2, unit: 'm' }],
  ['lantern', { axis: 'height', value: 1.2, unit: 'm' }],
];

export function suggestFor(name: string, kind: AssetKind): Suggestion | null {
  if (kind === 'scene') return null;
  if (kind === 'character') {
    return { size: { axis: 'height', value: 170, unit: 'cm' }, matched: 'character' };
  }
  const text = ` ${name.toLowerCase().replace(/[^a-z]+/g, ' ')} `;
  const hit = TABLE.find(([word]) => text.includes(` ${word} `) || text.includes(` ${word}s `));
  return hit ? { size: hit[1], matched: hit[0] } : null;
}

/** The core's checks, with its wording. */
export function checkPrep(entry: ModelPrep, what: string): void {
  const range = (ok: boolean, message: string) => ok || fail('invalid', `${what}: ${message}`);
  if (entry.size) {
    const m = metres(entry.size);
    range(
      Number.isFinite(m) && m >= MIN_METRES && m <= MAX_METRES,
      'the real size must be between 0.1 cm and 1000 m.'
    );
  }
  const seen = new Set<string>();
  for (const f of entry.fixups) {
    range(!seen.has(f.op), `${FIXUP_LABEL[f.op].toLowerCase()} is there twice.`);
    seen.add(f.op);
    if (f.op === 'decimate') {
      range(
        f.triangles >= 100 && f.triangles <= 10_000_000,
        'decimate to 100 to 10,000,000 triangles.'
      );
    }
    if (f.op === 'shrink-textures') {
      range(f.maxSize >= 64 && f.maxSize <= 16_384, 'shrink textures to 64 to 16384 px.');
    }
    if (f.op === 'remove-loose') {
      const share = f.minShare ?? -1;
      range(share >= 0 && share <= 0.5, 'loose parts are 0 to 50% of the model.');
    }
  }
}

const tidy = (entry: ModelPrep): ModelPrep => ({
  size: entry.size,
  fixups: [...entry.fixups].sort((a, b) => FIXUP_ORDER.indexOf(a.op) - FIXUP_ORDER.indexOf(b.op)),
  skipBake: entry.skipBake ?? null,
  padEdges: entry.padEdges ?? null,
});

/** One fix-up's effect on the numbers, with the script's note. */
function step(f: Fixup, n: ModelNumbers): FixupStep {
  const after: ModelNumbers = { ...n };
  let note = '';
  if (f.op === 'apply-transforms') note = `${n.objects} meshes`;
  if (f.op === 'remove-loose') {
    after.triangles = Math.max(0, n.triangles - Math.round(n.triangles * 0.004));
    note = n.objects > 1 ? '2 parts under 3 cm' : 'no loose parts';
  }
  if (f.op === 'merge') {
    after.objects = 1;
    note = n.objects > 1 ? `${n.objects} meshes joined into 1` : 'nothing to join';
  }
  if (f.op === 'decimate') {
    after.triangles = Math.min(n.triangles, f.triangles);
    note = `ratio ${(after.triangles / Math.max(1, n.triangles)).toFixed(3)}`;
  }
  if (f.op === 'shrink-textures') {
    after.maxTexture = Math.min(n.maxTexture, f.maxSize);
    note = n.maxTexture > f.maxSize ? `textures to ${f.maxSize} px` : 'all at or under';
  }
  if (f.op === 'origin-bottom' && n.min && n.max) {
    const frame = frameOf({ size: null, fixups: [f] }, n.min, n.max);
    after.min = framed(frame, n.min);
    after.max = framed(frame, n.max);
    note = 'moved to the floor';
  }
  return { op: f.op, before: n, after, note };
}

/** The numbers a run of `plan` on a model reports. */
export function previewReport(
  plan: ModelPrep,
  before: ModelNumbers,
  bytes: number | null,
  output: string
): FixupReport {
  const steps: FixupStep[] = [];
  let now = before;
  for (const f of plan.fixups) {
    const done = step(f, now);
    steps.push(done);
    now = done.after;
  }
  const extent = now.min && now.max ? extentOf(now.min, now.max) : null;
  const scale = plan.size && extent ? scaleFor(plan.size, extent) : null;
  if (scale && now.min && now.max) {
    const frame = { scale, offset: [0, 0, 0] as [number, number, number] };
    now = { ...now, min: framed(frame, now.min), max: framed(frame, now.max) };
  }
  const tris = now.triangles / Math.max(1, before.triangles);
  const tex = before.maxTexture ? (now.maxTexture / before.maxTexture) ** 2 : 1;
  return {
    plan,
    before,
    after: now,
    steps,
    scale,
    bytesBefore: bytes,
    bytesAfter: bytes === null ? null : Math.round(bytes * (0.35 * tris + 0.65 * tex)),
    output,
  };
}

interface PrepareDeps {
  project: (id: string) => ProjectManifest;
  config: () => WorkspaceConfig;
  inbox: () => InboxItem[];
  /** Bounds of a project file, when mock mode knows them. */
  bounds: (file: string) => Bounds | null;
  probeDelay: number;
}

export function createPrepareHandlers(deps: PrepareDeps) {
  const stored = new Map<string, ScenePrep>();
  // What accepted inbox files measure, so their real size has a model to scale.
  const accepted = new Map<string, Bounds>();
  const bounds = (file: string): Bounds | null => accepted.get(file) ?? deps.bounds(file);
  const vec = (v: readonly (number | null)[]) => [v[0] ?? 0, v[1] ?? 0, v[2] ?? 0] as Bounds['min'];
  const get = (id: string): ScenePrep => stored.get(id) ?? { version: 1, models: {} };
  const defaults = (): FixupDefaults => ({
    triangles: deps.config().budgets.trisPerModel,
    maxTexture: 2048,
    looseShare: 0.02,
  });

  function latestOf(projectId: string, assetId: string) {
    const asset = (deps.project(projectId).assets ?? []).find(a => a.id === assetId);
    if (!asset) fail('not-found', `Could not find the asset ${assetId}.`);
    return { asset, latest: asset.versions[asset.versions.length - 1] ?? null };
  }

  function numbers(projectId: string, assetId: string): ModelNumbers | null {
    const { latest } = latestOf(projectId, assetId);
    if (!latest) return null;
    const b = bounds(latest.file);
    return {
      triangles: latest.stats?.triangles ?? 40_000,
      objects: 1,
      maxTexture: latest.stats?.maxTexture ?? 2048,
      min: b?.min ?? null,
      max: b?.max ?? null,
    };
  }

  function view(projectId: string, assetId: string): AssetPrep {
    const { asset, latest } = latestOf(projectId, assetId);
    const entry = get(projectId).models[asset.id] ?? { size: null, fixups: [], skipBake: null };
    const n = numbers(projectId, asset.id);
    const measured = n?.min && n.max ? extentOf(n.min, n.max) : null;
    const report =
      n && (entry.size || entry.fixups.length)
        ? previewReport(entry, n, latest?.stats?.bytes ?? null, '')
        : null;
    const last = entry.fixups.length ? report?.steps.at(-1)?.after : null;
    const fixed = last?.min && last.max ? extentOf(last.min, last.max) : null;
    const base = fixed ?? measured;
    return {
      project: projectId,
      asset: asset.id,
      name: asset.name,
      kind: asset.kind,
      entry,
      version: latest?.version ?? null,
      measured,
      fixed,
      scale: entry.size && base ? scaleFor(entry.size, base) : null,
      suggestion: suggestFor(asset.name, asset.kind),
      report: null,
      defaults: defaults(),
    };
  }

  /** Replaces one asset's entry, as Accept and a save do. */
  function saveEntry(projectId: string, asset: string, entry: ModelPrep): void {
    checkPrep(entry, asset);
    const models = { ...get(projectId).models };
    const chosen = (entry.skipBake ?? null) !== null || (entry.padEdges ?? null) !== null;
    if (entry.size || entry.fixups.length || chosen) models[asset] = tidy(entry);
    else delete models[asset];
    stored.set(projectId, { version: 1, models });
  }

  return {
    /** What `prepare.json` holds for a project. */
    entries: get,
    saveEntry,
    bounds,
    check: checkPrep,
    remember: (file: string, item: InboxItem) => {
      const g = item.report;
      if (g?.min && g.max) accepted.set(file, { min: vec(g.min), max: vec(g.max) });
    },
    handlers: {
      prepare_save: ({ project, asset, entry }: Args) => {
        latestOf(String(project), String(asset));
        saveEntry(String(project), String(asset), entry as ModelPrep);
        return view(String(project), String(asset));
      },
      prepare_get: ({ project }: Args) => {
        deps.project(String(project));
        return get(String(project));
      },
      prepare_asset: ({ project, asset }: Args) => view(String(project), String(asset)),
      prepare_suggest: ({ name, kind }: Args) => suggestFor(String(name), kind as AssetKind),
      prepare_preview: async ({ request }: Args) => {
        const r = request as PreviewRequest;
        checkPrep(r.plan, 'The plan');
        await sleep(deps.probeDelay * 2);
        if (r.inbox) {
          const item = deps.inbox().find(i => i.id === r.inbox);
          if (!item?.report) fail('not-found', `${r.inbox} in the inbox`);
          const g = item.report;
          const before: ModelNumbers = {
            triangles: g.triangles,
            objects: g.objects ?? g.meshes,
            maxTexture: g.maxTexture,
            min: g.min ?? null,
            max: g.max ?? null,
          };
          return previewReport(r.plan, before, g.bytes, `/mock/cache/fixups/${item.id}`);
        }
        const n = numbers(String(r.project), String(r.asset));
        if (!n) fail('invalid', 'This asset has no model yet, so there is nothing to fix.');
        const { latest } = latestOf(String(r.project), String(r.asset));
        return previewReport(r.plan, n, latest?.stats?.bytes ?? null, '/mock/cache/fixups/asset');
      },
    } as Record<string, (args: Args) => unknown>,
  };
}
