/**
 * Export packs in mock mode: the models a source holds, the presets, what
 * each model's check finds, and the packs a library at scale starts with
 * (one done, one Maquette closed on). The outcomes are set, not random,
 * so a run looks the same every time: the fourth model loses thin parts
 * at its budget, the seventh misses a texture, and a model on a Mixamo
 * rig keeps its bones and leaves its clips out.
 */
import type {
  AssetKind,
  LibraryItem,
  PackItem,
  PackModel,
  PackPreset,
  PackRecord,
  PackSettings,
  ProjectManifest,
} from '../client';

export const DEFAULT_SETTINGS: PackSettings = {
  name: 'library-pack',
  format: 'obj',
  realSize: true,
  pivotBottom: true,
  oneMaterial: true,
  texture: 1024,
  riggedGlb: true,
  compression: 'none',
  budgets: [
    { kind: 'prop', triangles: 3000 },
    { kind: 'character', triangles: null },
    { kind: 'scene', triangles: 20000 },
  ],
  readme: true,
  manifest: true,
  contactSheet: true,
  refs: false,
  zip: true,
};

export const BUILT_IN: PackPreset = {
  id: 'plain-obj',
  name: 'Plain OBJ pack',
  settings: DEFAULT_SETTINGS,
  builtIn: true,
  savedAt: null,
  used: 0,
};

const key = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '') || 'model';

/** A number from a name, the same every time. */
const hash = (text: string) => [...text].reduce((h, c) => (h * 31 + c.charCodeAt(0)) >>> 0, 7);

const LICENCE: Record<string, string> = {
  tripo: 'Made with Tripo. What you may do with it is set by the Tripo plan it was made on.',
  meshy: 'Made with Meshy. What you may do with it is set by the Meshy plan it was made on.',
  'by-hand': 'Made by hand on a provider site; its terms apply.',
  built: 'Built by a Blender script in the project.',
  imported: 'Brought in from a file; check where it came from before sharing it.',
};

/** Unique keys in order: `stool`, `stool_2`. */
function keyed(models: Omit<PackModel, 'key'>[]): PackModel[] {
  const taken = new Set<string>();
  return models.map(m => {
    let k = key(m.name);
    for (let n = 2; taken.has(k); n++) k = `${key(m.name)}_${n}`;
    taken.add(k);
    return { ...m, key: k };
  });
}

export function modelsOfItems(items: readonly LibraryItem[]): PackModel[] {
  return keyed(
    items.map(item => {
      const v = item.entry.versions[item.entry.versions.length - 1]!;
      const rigged = (v.stats?.joints ?? 0) > 0;
      const thumb = [...item.entry.versions].reverse().find(x => x.thumbnail)?.thumbnail;
      return {
        name: item.entry.name,
        kind: item.entry.kind,
        entry: item.entry.id,
        project: null,
        asset: null,
        version: v.version,
        file: `${item.dir}/${v.file}`,
        stats: v.stats,
        rigged,
        heightM: 0.3 + (hash(item.entry.id) % 180) / 100,
        mixamo: rigged && v.source !== 'tripo',
        source: v.source,
        licence: LICENCE[v.source] ?? '',
        thumbnail: thumb ? `${item.dir}/${thumb}` : (item.entry.refs[0] ?? null),
        refs: [],
        prep: v.prep,
      };
    })
  );
}

export function modelsOfProject(project: ProjectManifest, dir: string): PackModel[] {
  const accepted = (project.assets ?? []).filter(a => a.versions.length > 0);
  return keyed(
    accepted.map(a => {
      const v = a.versions[a.versions.length - 1]!;
      const rigged = (v.stats?.joints ?? 0) > 0;
      return {
        name: a.name,
        kind: a.kind,
        entry: null,
        project: project.id,
        asset: a.id,
        version: v.version,
        file: `${dir}/${v.file}`,
        stats: v.stats,
        rigged,
        heightM: 0.3 + (hash(a.id) % 180) / 100,
        mixamo: rigged,
        source: null,
        licence: '',
        thumbnail: a.thumbnail ? `${dir}/${a.thumbnail}` : null,
        refs: [],
        prep: null,
      };
    })
  );
}

export function budgetOf(settings: PackSettings, kind: AssetKind): number | null {
  return settings.budgets.find(b => b.kind === kind)?.triangles ?? null;
}

export function newItem(model: PackModel, budget: number | null): PackItem {
  return {
    model,
    state: 'queued',
    budget,
    triangles: null,
    heightM: null,
    files: [],
    picture: null,
    note: null,
    suggest: null,
    clipsLeftOut: 0,
    tookMs: 0,
    bytes: null,
  };
}

const fmt = (n: number) => Math.round(n).toLocaleString('en-US');

/** The files a model becomes under `settings`. */
function filesOf(item: PackItem, s: PackSettings): string[] {
  const k = item.model.key;
  const files: string[] = [];
  if (s.format !== 'glb') files.push(`${k}/${k}.obj`, `${k}/${k}.mtl`, `${k}/${k}.png`);
  if (s.format !== 'obj' || (item.model.rigged && s.riggedGlb)) files.push(`${k}/${k}.glb`);
  return files;
}

/** What the check finds on run `attempt` (0 first) of the model at `index`. */
export function outcome(
  item: PackItem,
  index: number,
  attempt: number,
  settings: PackSettings,
  tookMs: number
): PackItem {
  const before = item.model.stats?.triangles ?? 0;
  const budget = item.budget;
  const decimated = budget !== null && before > budget && !item.model.rigged;
  const triangles = decimated ? budget : before;
  const base: PackItem = {
    ...item,
    triangles,
    heightM: settings.realSize ? item.model.heightM : null,
    files: filesOf(item, settings),
    picture: item.model.thumbnail,
    tookMs,
    bytes: triangles * 40 + 300_000,
    suggest: null,
  };
  if (index === 6 && attempt === 0) {
    return {
      ...base,
      state: 'fail',
      files: [],
      triangles: null,
      note: `Texture missing: ${item.model.key}.png did not load when opened again`,
    };
  }
  if (index % 8 === 3 && attempt === 0 && decimated) {
    const suggest = Math.min(before, budget * 2);
    return {
      ...base,
      state: 'warn',
      note: `Thin parts lost at ${fmt(budget)}; ${fmt(suggest)} keeps them`,
      suggest,
    };
  }
  if (item.model.rigged && budget !== null && before > budget) {
    return {
      ...base,
      state: 'warn',
      note: `Rigged, so not decimated: ${fmt(before)} over the ${fmt(budget)} budget`,
    };
  }
  const clips = item.model.mixamo ? Math.max(1, item.model.stats?.animations ?? 1) : 0;
  const said =
    before <= (budget ?? Infinity) ? 'Under budget, left as is' : 'Re-imported, Y up, on the floor';
  return {
    ...base,
    state: 'pass',
    clipsLeftOut: clips,
    note: clips ? `${said}; ${clips} Mixamo clip${clips === 1 ? '' : 's'} left out` : said,
  };
}

/** The packs a library at scale starts with: one done, one Maquette closed on. */
export function seedRecords(
  items: readonly LibraryItem[],
  root: string,
  nowMs: number
): PackRecord[] {
  if (items.length < 18) return [];
  const iso = (ms: number) => new Date(ms).toISOString();
  const record = (name: string, from: number, n: number, ageMs: number): PackRecord => ({
    version: 1,
    id: `${name}-${iso(nowMs - ageMs)
      .replace(/\D/g, '')
      .slice(0, 14)}`,
    name,
    label: `${n} models from the library`,
    source: { from: 'entries', entries: items.slice(from, from + n).map(i => i.entry.id) },
    settings: { ...DEFAULT_SETTINGS, name },
    dir: `${root}/exports/${name}`,
    zip: null,
    state: 'done',
    items: modelsOfItems(items.slice(from, from + n)).map(m =>
      newItem(m, budgetOf(DEFAULT_SETTINGS, m.kind))
    ),
    createdAt: iso(nowMs - ageMs),
    finishedAt: iso(nowMs - ageMs + 192_000),
    bytes: null,
    message: null,
    leftOut: 0,
    preset: 'plain-obj',
  });
  const done = record('tavern-kit', 30, 9, 2 * 86_400_000);
  done.items = done.items.map((i, n) => ({
    ...outcome(i, n + 10, 1, done.settings, 16_000),
    state: 'pass',
  }));
  done.zip = `${done.dir}.zip`;
  done.bytes = 9_800_000;
  const closed = record('market-set', 50, 18, 5 * 3_600_000);
  closed.state = 'stopped';
  closed.finishedAt = null;
  closed.message = 'Maquette closed with 6 of 18 done, model 7 is next';
  closed.items = closed.items.map((i, n) =>
    n < 6 ? { ...outcome(i, n + 10, 1, closed.settings, 15_000), state: 'pass' } : i
  );
  return [closed, done];
}
