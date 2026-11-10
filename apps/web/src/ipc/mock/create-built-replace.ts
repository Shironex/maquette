/**
 * A built asset in the place of another model, in mock mode. The asset
 * keeps its id and gains the built asset's source and a new version; the
 * built asset's own card leaves the Board and is kept here for the undo, as
 * the app keeps it in the asset trash. `?built=swap` adds Lantern rebuilt, a
 * built asset ready to take Crystal lantern's place, and `?built=swapped`
 * opens with that already done.
 */
import type {
  Asset,
  BuiltSource,
  ModelPrep,
  Placement,
  ProjectManifest,
  ReplaceDone,
  ReplaceReport,
  ShipSwap,
  SwapSetting,
  SwapSide,
  UndoDone,
  UndoReport,
} from '../client';
import { clock } from '@/lib/clock';
import { extentOf, scaleFor } from '@/lib/real-size';
import { describeFixup, describeSize } from '@/state/prepare';
import { fail, type Args } from './backend-options';

type Vec3 = [number, number, number];

export interface ReplaceContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  /** Tells the UI the project's built assets changed. */
  emit: (project: string, manifest?: ProjectManifest) => void;
  /** A project's built sources, by asset id. */
  sources: (project: string) => Map<string, BuiltSource>;
  /** How a built asset stands; only a current one takes a place. */
  stateOf: (project: string, asset: string) => string | null;
  prep: (project: string, asset: string) => ModelPrep | null;
  savePrep: (project: string, asset: string, entry: ModelPrep) => void;
  /** Where the layout places the asset, when it does. */
  placement: (project: string, asset: string) => Placement | null;
  /** Bounds of a project file, when mock mode knows them. */
  bounds: (file: string) => { min: Vec3; max: Vec3 } | null;
}

const NOTE =
  'Nothing is aligned. The layout keeps the asset’s position, turn and scale, so the built model has to face the way the old model stood and sit around the same origin: compare the old and new bounds.';

const UNIT: { min: Vec3; max: Vec3 } = { min: [-0.5, 0, -0.5], max: [0.5, 1, 0.5] };

const stem = (file: string) => (file.split('/').pop() ?? file).replace(/\.[^.]+$/, '');

const latest = (asset: Asset) => asset.versions[asset.versions.length - 1];

const triple = (v: readonly (number | null)[]) => v.map(n => n ?? 0).join(', ');

function scaled(box: { min: Vec3; max: Vec3 }, by: number): { min: Vec3; max: Vec3 } {
  const times = (v: Vec3): Vec3 => [v[0] * by, v[1] * by, v[2] * by];
  return { min: times(box.min), max: times(box.max) };
}

/** The card a replace took off the Board, kept for the undo. */
interface Retired {
  card: Asset;
  index: number;
  source: BuiltSource;
}

export function createBuiltReplace(ctx: ReplaceContext) {
  const retired = new Map<string, Retired>();

  function pair(id: string, assetId: string, builtId: string) {
    const p = ctx.project(id);
    const find = (wanted: string) => {
      const found = (p.assets ?? []).find(a => a.id === wanted);
      if (!found) fail('not-found', `Could not find the asset ${wanted}.`);
      return found;
    };
    const asset = find(assetId);
    const by = find(builtId);
    if (asset.id === by.id)
      fail('invalid', 'Pick two assets: the built asset, and the one whose model it replaces.');
    const sources = ctx.sources(id);
    if (sources.has(asset.id))
      fail(
        'invalid',
        `${asset.name} is already a built asset. Edit its script in built/ and rebuild it.`
      );
    const source = sources.get(by.id);
    if (!source)
      fail('invalid', `${by.name} is not a built asset. Add its script as a built asset first.`);
    const old = latest(asset);
    if (!old)
      fail(
        'invalid',
        `${asset.name} has no model yet, so there is nothing to replace. Use ${by.name} as it is.`
      );
    const state = ctx.stateOf(id, by.id);
    if (state === 'stale')
      fail(
        'invalid',
        `${by.name} has a script that changed since its last build. Rebuild it first, so the model that takes the place is the one its script makes.`
      );
    if (state !== 'current') fail('invalid', `${by.name} has not built yet. Build it first.`);
    const from = latest(by);
    if (!from) fail('invalid', `${by.name} has no model to bring.`);
    return { p, asset, by, source, old, from };
  }

  function report(id: string, assetId: string, builtId: string): ReplaceReport {
    const { asset, by, source, old, from } = pair(id, assetId, builtId);
    const prep = ctx.prep(id, asset.id);
    const box = ctx.bounds(old.file) ?? UNIT;
    const scale = prep?.size ? (scaleFor(prep.size, extentOf(box.min, box.max)) ?? 1) : 1;
    const stands = scaled(box, scale);
    const number = Math.max(...asset.versions.map(v => v.version)) + 1;
    const file = `models/${asset.id.replace(/-/g, '_')}_v${number}.glb`;
    const side = (version: number, path: string, stats: Asset['versions'][number]['stats']) => ({
      version,
      file: path,
      shipsAs: `${stem(path)}.glb`,
      triangles: stats?.triangles ?? null,
      bytes: stats?.bytes ?? null,
    });
    const oldSide: SwapSide = { ...side(old.version, old.file, old.stats), ...stands, scale };
    // The mock's built model is built at the real size, around the same origin.
    const newSide: SwapSide = { ...side(number, file, from.stats), ...stands, scale: 1 };
    const settings: SwapSetting[] = [];
    if (prep?.size)
      settings.push({
        what: `Real size, ${describeSize(prep.size)}`,
        action: 'carried',
        why: 'It is a measure, so the built model is scaled to it from its own bounds (by 1 here) and never twice.',
      });
    for (const fixup of prep?.fixups ?? [])
      settings.push({
        what: describeFixup(fixup),
        action: 'reset',
        why: 'It repaired the old model’s mesh or textures.',
      });
    if (prep?.skipBake)
      settings.push({
        what: 'Skips the bake',
        action: 'reset',
        why: 'A generated scan skips the bake. A built model bakes like every other built piece.',
      });
    if (prep?.padEdges)
      settings.push({
        what: 'Pads texture edges',
        action: 'reset',
        why: 'It fills the gaps of a scan’s texture atlas.',
      });
    const placed = ctx.placement(id, asset.id);
    if (placed)
      settings.push({
        what: `Layout placement: position ${triple(placed.position)}, turn ${triple(placed.rotation)}, scale ${placed.scale ?? 1}`,
        action: 'kept',
        why: 'It is keyed by the asset’s id, which stays.',
      });
    return {
      project: id,
      asset: asset.id,
      name: asset.name,
      built: by.id,
      builtName: by.name,
      script: source.script,
      old: oldSide,
      new: newSide,
      bakes: true,
      parts: source.last?.objects ?? [],
      settings,
      unresolved: [],
      warnings: prep?.size
        ? []
        : [
            `No real size is set for ${asset.name}, so the built model stands at its own size. Set the real size, or build it to that size.`,
          ],
      note: NOTE,
      trash: null,
    };
  }

  function run(id: string, assetId: string, builtId: string): ReplaceDone {
    const planned = report(id, assetId, builtId);
    const { p, asset, by, source, from } = pair(id, assetId, builtId);
    const at = clock.now().toISOString();
    const prep = ctx.prep(id, asset.id);
    const trash = `${by.id}-${at.replace(/\D/g, '').slice(0, 17)}`;
    const assets = p.assets ?? [];
    retired.set(`${id}/${asset.id}`, { card: by, index: assets.indexOf(by), source });
    const version = {
      ...from,
      version: planned.new.version,
      file: planned.new.file,
      createdAt: at,
    };
    const saved = ctx.saveProject({
      ...p,
      assets: assets
        .filter(a => a.id !== by.id)
        .map(a =>
          a.id === asset.id
            ? {
                ...a,
                versions: [...a.versions, version],
                status: 'done' as const,
                origin: null,
                updatedAt: at,
              }
            : a
        ),
    });
    const sources = ctx.sources(id);
    sources.delete(by.id);
    sources.set(asset.id, {
      ...source,
      last: source.last ? { ...source.last, version: version.version } : null,
      replaced: {
        at,
        by: { asset: by.id, name: by.name, index: assets.indexOf(by), trash },
        versions: asset.versions.map(v => v.version),
        thumbnail: asset.thumbnail,
        origin: asset.origin ?? null,
        status: asset.status,
        stage: asset.stage,
        prep,
        materials: null,
      },
    });
    if (prep) ctx.savePrep(id, asset.id, { ...prep, fixups: [], skipBake: null, padEdges: null });
    ctx.emit(id, saved);
    return { report: { ...planned, trash }, project: saved };
  }

  function undoReport(id: string, assetId: string): UndoReport {
    const p = ctx.project(id);
    const asset = (p.assets ?? []).find(a => a.id === assetId);
    if (!asset) fail('not-found', `Could not find the asset ${assetId}.`);
    const source = ctx.sources(id).get(asset.id);
    if (!source) fail('invalid', `${asset.name} is not a built asset.`);
    const record = source.replaced;
    if (!record)
      fail(
        'invalid',
        `${asset.name} was added as a built asset and took no other model’s place, so there is no older model to bring back.`
      );
    const ref = (v: Asset['versions'][number]) => ({
      version: v.version,
      file: v.file,
      triangles: v.stats?.triangles ?? null,
      bytes: v.stats?.bytes ?? null,
    });
    const kept = asset.versions.filter(v => record.versions.includes(v.version));
    const back = kept[kept.length - 1];
    if (!back)
      fail(
        'invalid',
        `The model ${asset.name} had before is no longer among its versions, so it cannot come back.`
      );
    const prep = record.prep ?? null;
    return {
      project: id,
      asset: asset.id,
      name: asset.name,
      script: source.script,
      back: ref(back),
      takenOff: asset.versions.filter(v => !record.versions.includes(v.version)).map(ref),
      setAside: null,
      restores: retired.has(`${id}/${asset.id}`) ? record.by.name : null,
      settings: [
        ...(prep?.size ? [`Real size, ${describeSize(prep.size)}`] : []),
        ...(prep?.fixups ?? []).map(describeFixup),
        ...(prep?.skipBake ? ['Skips the bake'] : []),
        ...(prep?.padEdges ? ['Pads texture edges'] : []),
      ],
      warnings: [],
    };
  }

  function undo(id: string, assetId: string): UndoDone {
    const planned = undoReport(id, assetId);
    const p = ctx.project(id);
    const sources = ctx.sources(id);
    const record = sources.get(assetId)?.replaced;
    if (!record) fail('invalid', `${planned.name} is not a built asset.`);
    const was = retired.get(`${id}/${assetId}`);
    const assets = (p.assets ?? []).map(a =>
      a.id === assetId
        ? {
            ...a,
            versions: a.versions.filter(v => record.versions.includes(v.version)),
            thumbnail: record.thumbnail,
            origin: record.origin ?? null,
            status: record.status,
            stage: record.stage,
            updatedAt: clock.now().toISOString(),
          }
        : a
    );
    if (was) assets.splice(Math.min(was.index, assets.length), 0, was.card);
    const saved = ctx.saveProject({ ...p, assets });
    sources.delete(assetId);
    if (was) sources.set(was.card.id, was.source);
    retired.delete(`${id}/${assetId}`);
    if (record.prep) ctx.savePrep(id, assetId, record.prep);
    ctx.emit(id, saved);
    return {
      report: {
        ...planned,
        setAside: `trash/${id}/${assetId}-${record.at.replace(/\D/g, '').slice(0, 17)}`,
      },
      project: saved,
    };
  }

  /** Every replace of the project: mock mode never ships one away. */
  function swaps(id: string): ShipSwap[] {
    const p = ctx.project(id);
    return [...ctx.sources(id)].flatMap(([assetId, source]) => {
      const asset = (p.assets ?? []).find(a => a.id === assetId);
      const record = source.replaced;
      if (!asset || !record) return [];
      const old = asset.versions.filter(v => record.versions.includes(v.version)).pop();
      const now = latest(asset);
      if (!old || !now || old.version === now.version) return [];
      return [
        {
          asset: asset.id,
          name: asset.name,
          script: source.script,
          at: record.at,
          oldFile: `${stem(old.file)}.glb`,
          newFile: `${stem(now.file)}.glb`,
          oldVersion: old.version,
          newVersion: now.version,
          oldTriangles: old.stats?.triangles ?? null,
          newTriangles: now.stats?.triangles ?? null,
          oldBytes: old.stats?.bytes ?? null,
          newBytes: now.stats?.bytes ?? null,
        },
      ];
    });
  }

  const handlers: Record<string, (args: Args) => unknown> = {
    built_replace_plan: ({ project, asset, built }) =>
      report(String(project), String(asset), String(built)),
    built_replace: ({ project, asset, built }) =>
      run(String(project), String(asset), String(built)),
    built_replace_undo_plan: ({ project, asset }) => undoReport(String(project), String(asset)),
    built_replace_undo: ({ project, asset }) => undo(String(project), String(asset)),
    built_swaps: ({ project }) => swaps(String(project)),
  };
  return { handlers, run };
}
