/**
 * Parts in mock mode: an asset's parts as its newest model ships them, the
 * names, joins and hidden parts `prepare.json` keeps for them, and two
 * versions side by side the way Compare shows them.
 *
 * `?parts=demo` gives Frostveil a frost lantern that came from Tripo as one
 * mesh (v2) with a split in Blender added, its five parts named, and a rune
 * chest that is still one mesh. The Inbox gains a second frost lantern
 * download, one mesh, with the "One mesh, no parts" check.
 */
import type {
  Asset,
  AssetParts,
  InboxItem,
  ModelParts,
  ModelPrep,
  PartRow,
  PartsCompare,
  PartsPlan,
  PartsSide,
  ProjectManifest,
} from '../client';
import { clock } from '@/lib/clock';
import { expand, fail, type Args } from './backend-options';
import { DEFAULT_ROOT } from './fixtures';
import { thumbsDir } from './inbox-fixtures';
import { isEmptyPlan, planOf } from '@/state/parts';

/** A part as the model file has it. */
interface PartInfo {
  name: string;
  triangles: number;
  materials: string[];
}

interface Sample {
  /** The model as it came. */
  whole: PartInfo;
  /** What the split leaves, top first. */
  parts: PartInfo[];
  pieces: number;
  /** The nodes an assembled prop already ships, one part each. */
  nodes?: PartInfo[];
}

const part = (name: string, triangles: number, ...materials: string[]): PartInfo => ({
  name,
  triangles,
  materials,
});

/** The bundled models' parts, with the numbers a Tripo prop has. */
const SAMPLES: Record<string, Sample> = {
  'frost-lantern': {
    whole: part('frost_lantern', 12_200, 'stone', 'iron', 'crystal_glow', 'granite'),
    parts: [
      part('part_1', 2_400, 'stone'),
      part('part_2', 2_900, 'iron'),
      part('part_3', 1_200, 'crystal_glow'),
      part('part_4', 3_100, 'stone'),
      part('part_5', 2_600, 'granite'),
    ],
    pieces: 9,
  },
  'sigil-gate': {
    whole: part('sigil_gate', 32_400, 'veil_stone', 'sigil_glow'),
    parts: [],
    pieces: 3,
    nodes: [
      part('frame', 21_400, 'veil_stone'),
      part('door_leaf', 6_200, 'veil_stone', 'sigil_glow'),
      part('steps', 4_800, 'veil_stone'),
    ],
  },
  'rune-chest': {
    whole: part('rune_chest', 8_600, 'rune_wood', 'frost_iron'),
    parts: [part('part_1', 3_100, 'rune_wood', 'frost_iron'), part('part_2', 5_500, 'rune_wood')],
    pieces: 2,
  },
};

/** Project files to the bundled model each one serves, for `?parts=`. */
export function partsFiles(parts: string | null): Record<string, string> {
  if (!parts) return {};
  return {
    'models/frost_lantern_v1.glb': 'frost_lantern.glb',
    'models/frost_lantern_v2.glb': 'frost_lantern.glb',
    'models/rune_chest_v1.glb': 'rune_chest.glb',
  };
}

/** Fix-up preview copies to the bundled model each one serves. */
export const PARTS_CACHE: Record<string, string> = {
  '/mock/cache/fixups/frost-lantern.glb': 'frost_lantern_split.glb',
  '/mock/cache/fixups/frost_lantern.glb': 'frost_lantern_split.glb',
  '/mock/cache/fixups/rune-chest.glb': 'frost_lantern_split.glb',
};

function sampleAsset(id: string, name: string, versions: number, kb: number): Asset {
  const at = new Date(clock.nowMs() - 5 * 3_600_000).toISOString();
  const stem = id.replace(/-/g, '_');
  const whole = SAMPLES[id]?.whole.triangles ?? 1_000;
  return {
    id,
    name,
    kind: 'prop',
    stage: 'blender',
    status: 'review',
    versions: Array.from({ length: versions }, (_, i) => ({
      version: i + 1,
      file: `models/${stem}_v${i + 1}.glb`,
      createdAt: at,
      stats: {
        triangles: whole,
        bytes: kb * 1000,
        textures: 1,
        maxTexture: 2048,
        joints: 0,
        animations: 0,
      },
    })),
    thumbnail: id === 'frost-lantern' ? 'thumbs/crystal-lantern.webp' : null,
    createdAt: at,
    updatedAt: at,
  };
}

/** Frostveil with the frost lantern and the rune chest, when `?parts=` asks. */
export function seedPartsProjects(
  projects: ProjectManifest[],
  parts: string | null
): ProjectManifest[] {
  if (!parts) return projects;
  const extra = [
    sampleAsset('frost-lantern', 'Frost lantern', 2, 1_840),
    sampleAsset('rune-chest', 'Rune chest', 1, 960),
  ];
  return projects.map(p =>
    p.id === 'frostveil' ? { ...p, assets: [...extra, ...(p.assets ?? [])] } : p
  );
}

/** The frost lantern's split, named the way the Inspector shows it. */
export const LANTERN_PREP: ModelPrep = {
  size: null,
  fixups: [{ op: 'split-parts', by: 'loose', mergeUnder: 0.01 }],
  skipBake: null,
  padEdges: null,
  parts: {
    names: { part_1: 'lid', part_2: 'cage', part_3: 'crystal', part_4: 'post', part_5: 'base' },
    merge: [],
    hidden: ['part_5'],
  },
};

/** A second frost lantern download, still one mesh. */
export function partsInboxItem(dir: string, now: number): InboxItem {
  return {
    id: 'frost_lantern.glb',
    name: 'Frost lantern',
    file: `${dir}/frost_lantern.glb`,
    bytes: 1.84e6,
    modifiedMs: now - 2 * 60_000,
    kind: 'prop',
    kindGuessed: true,
    kindFromSend: false,
    state: 'ready',
    report: {
      min: [-0.18, 0, -0.18],
      max: [0.18, 0.86, 0.18],
      objects: 1,
      transformed: 0,
      bytes: 1.84e6,
      generator: 'Tripo',
      triangles: 12_200,
      vertices: 7_400,
      meshes: 1,
      primitives: 4,
      materials: 4,
      textures: [],
      maxTexture: 2048,
      skins: 0,
      joints: 0,
      jointNames: [],
      rig: 'none',
      animations: [],
      size: [0.36, 0.86, 0.36],
      pbr: {
        baseColor: true,
        metallicRoughness: true,
        normal: true,
        occlusion: false,
        emissive: false,
      },
      extensions: [],
      geometryReadable: true,
      islands: { mesh: 'frost_lantern', islands: 9, groups: 9 },
      fingerWeights: null,
    },
    checks: [
      { id: 'triangles', status: 'pass', title: '12.2k triangles', detail: null, fixes: [] },
      { id: 'rig', status: 'pass', title: 'No rig, as expected', detail: null, fixes: [] },
      {
        id: 'merged',
        status: 'warning',
        title: 'One mesh, no parts',
        detail:
          'Motion states and explode tracks move named parts. Split it so its 9 pieces can move on their own.',
        fixes: [
          { op: 'split-parts', by: 'loose', mergeUnder: 0.01 },
          { op: 'remove-loose', minShare: 0.02 },
        ],
      },
      {
        id: 'origin',
        status: 'pass',
        title: 'Origin at the bottom centre',
        detail: null,
        fixes: [],
      },
    ],
    worst: 'warning',
    error: null,
    thumbnail: `${thumbsDir(dir)}/crystal-lantern.webp`,
    thumbnailState: 'ready',
    aim: null,
    link: null,
  };
}

/** How many loose pieces an asset's newest model has. */
export const piecesOf = (asset: string): number | null => SAMPLES[asset]?.pieces ?? null;

const splits = (entry: ModelPrep | undefined) =>
  (entry?.fixups ?? []).some(f => f.op === 'split-parts' || f.op === 'merge');

/** The parts as they ship under `plan`, the way the core lists them. */
export function partRows(found: PartInfo[], given: PartsPlan): PartRow[] {
  const plan = planOf(given);
  const groupOf = (name: string) => plan.merge.find(g => g.includes(name));
  const rows: PartRow[] = [];
  for (const p of found) {
    const group = groupOf(p.name);
    if (group && group[0] !== p.name) continue;
    const joined = group ? group.slice(1).filter(n => found.some(f => f.name === n)) : [];
    const more = found.filter(f => joined.includes(f.name));
    rows.push({
      id: p.name,
      name: plan.names[p.name] ?? p.name,
      triangles: p.triangles + more.reduce((sum, f) => sum + f.triangles, 0),
      materials: [...new Set([...p.materials, ...more.flatMap(f => f.materials)])],
      hidden: plan.hidden.includes(p.name),
      joined,
    });
  }
  return rows;
}

const EMPTY_PLAN: PartsPlan = planOf(null);

type SaveEntry = (project: string, asset: string, entry: ModelPrep) => void;

/**
 * With `?parts=`: Frostveil gains the frost lantern and the rune chest, the
 * lantern its split, and the Inbox the second lantern.
 */
export function seedParts(
  parts: string | null,
  projects: ProjectManifest[],
  inbox: InboxItem[],
  saveEntry: SaveEntry
): [ProjectManifest[], InboxItem[]] {
  if (!parts) return [projects, inbox];
  saveEntry('frostveil', 'frost-lantern', LANTERN_PREP);
  const dir = `${expand(DEFAULT_ROOT)}/inbox`;
  return [seedPartsProjects(projects, parts), [partsInboxItem(dir, clock.nowMs()), ...inbox]];
}

interface PartsDeps {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  prepare: {
    entries: (project: string) => { models: Partial<Record<string, ModelPrep>> };
    saveEntry: SaveEntry;
  };
  /** The workspace folder, absolute; null before one is open. */
  root: () => string | null;
}

export function createPartsHandlers({ project: projectOf, saveProject, prepare, root }: PartsDeps) {
  const deps = {
    project: projectOf,
    saveProject,
    entry: (id: string, asset: string) => prepare.entries(id).models[asset],
    saveEntry: prepare.saveEntry,
    dir: (id: string) => `${root() ?? expand(DEFAULT_ROOT)}/projects/${id}`,
  };
  const previewed = new Set<string>();
  const assetOf = (project: string, id: string) => {
    const asset = (deps.project(project).assets ?? []).find(a => a.id === id || a.name === id);
    if (!asset) fail('not-found', `Could not find the asset ${id}.`);
    return asset;
  };

  function view(project: string, id: string): AssetParts {
    const asset = assetOf(project, id);
    const entry = deps.entry(project, asset.id);
    const latest = asset.versions.at(-1) ?? null;
    const sample = SAMPLES[asset.id];
    const split = splits(entry);
    const ran = previewed.has(`${project}/${asset.id}`);
    const plan = entry?.parts ?? EMPTY_PLAN;
    const found = sample ? (split ? sample.parts : (sample.nodes ?? [sample.whole])) : [];
    const file = split
      ? ran
        ? `/mock/cache/fixups/${asset.id}.glb`
        : null
      : latest
        ? `${deps.dir(project)}/${latest.file}`
        : null;
    return {
      project,
      asset: asset.id,
      name: asset.name,
      kind: asset.kind,
      version: latest?.version ?? null,
      file,
      split,
      needsPreview: split && !ran && latest !== null,
      rows: file ? partRows(found, plan) : [],
      plan,
      game: deps.project(project).target === 'game',
    };
  }

  /** One side's numbers: the split leaves more parts and draw calls, same triangles. */
  function side(project: string, asset: Asset, s: PartsSide): [ModelParts, string] {
    const version = asset.versions.find(v => v.version === s.version);
    if (!version) fail('invalid', `${asset.name} has no v${s.version}.`);
    const sample = SAMPLES[asset.id];
    const entry = deps.entry(project, asset.id);
    const fixed = s.fixed && splits(entry) && sample;
    const rows = sample
      ? partRows(fixed ? sample.parts : [sample.whole], entry?.parts ?? EMPTY_PLAN)
      : [];
    const stats = version.stats;
    const numbers: ModelParts = {
      parts: rows.length >= 2 ? rows.map(r => r.name) : [],
      drawCalls: fixed ? rows.length : (sample?.whole.materials.length ?? 1),
      triangles: stats?.triangles ?? 0,
      textures: stats?.textures ?? 0,
      maxTexture: stats?.maxTexture ?? 0,
      bytes: Math.round((stats?.bytes ?? 0) * (fixed ? 1.04 : 1)),
      checks: fixed ? { pass: 6, warning: 0, fail: 0 } : { pass: 5, warning: 1, fail: 0 },
    };
    const file = fixed
      ? `/mock/cache/fixups/${asset.id}.glb`
      : `${deps.dir(project)}/${version.file}`;
    return [numbers, file];
  }

  return {
    parts_get: ({ project, asset }: Args) => view(String(project), String(asset)),
    parts_preview: ({ project, asset }: Args) => {
      const found = assetOf(String(project), String(asset));
      previewed.add(`${String(project)}/${found.id}`);
      return view(String(project), found.id);
    },
    parts_save: ({ project, asset, parts }: Args) => {
      const id = assetOf(String(project), String(asset)).id;
      const entry = deps.entry(String(project), id) ?? { size: null, fixups: [], skipBake: null };
      const plan = parts as PartsPlan;
      deps.saveEntry(String(project), id, { ...entry, parts: isEmptyPlan(plan) ? null : plan });
      return view(String(project), id);
    },
    parts_compare: ({ project, asset, left, right }: Args): PartsCompare => {
      const found = assetOf(String(project), String(asset));
      const [l, leftFile] = side(String(project), found, left as PartsSide);
      const [r, rightFile] = side(String(project), found, right as PartsSide);
      return { left: l, right: r, leftFile, rightFile };
    },
    parts_keep_version: ({ project, asset, version }: Args) => {
      const p = deps.project(String(project));
      const found = assetOf(String(project), String(asset));
      const at = found.versions.findIndex(v => v.version === Number(version));
      if (at < 0) fail('invalid', `${found.name} has no v${Number(version)}.`);
      const versions = [...found.versions];
      versions.push(...versions.splice(at, 1));
      const assets = (p.assets ?? []).map(a => (a.id === found.id ? { ...a, versions } : a));
      return deps.saveProject({ ...p, assets });
    },
  } as Record<string, (args: Args) => unknown>;
}
