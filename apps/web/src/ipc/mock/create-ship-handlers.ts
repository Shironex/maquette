/**
 * The website Ship settings in mock mode: `ship.json`, the site folders, the
 * dry-run plan and the size report.
 *
 * The site holds `observatory_arch.glb` made by hand (not in the ship record) and
 * a `crystal_lantern.glb` Maquette shipped before that is smaller than the new one,
 * so the plan shows both holds. `snow_drift.glb` is the site's own model.
 * `?ship=hand-built` opens Frostveil already set to the models style with
 * WebP textures and the observatory arch set to never ship.
 */
import type {
  AssetShip_Serialize as Entry,
  ModelFile,
  PipelineView,
  ProjectManifest,
  ShipCheck,
  ShipSave_Deserialize as ShipSave,
  ShipSettings_Serialize as Settings,
  ShipView_Serialize as ShipView,
  SitePlanFile,
  SiteTarget,
  SizeRow,
} from '../client';
import { gameOf } from '@/state/game';
import { fail, type Args } from './backend-options';

interface ShipContext {
  project: (id: string) => ProjectManifest;
  view: (id: string) => PipelineView;
  state: string | null;
}

const DEFAULTS: Settings = {
  version: 1,
  style: 'scene',
  textures: 'ktx2',
  quality: 80,
  assets: {},
};
const LEGACY: SiteTarget = { models: 'public/models', posters: 'public/posters', data: null };
const HAND_MADE = new Set(['observatory_arch']);
const SMALLER = new Map([['crystal_lantern', 0.31e6]]);
const BEFORE = new Map([['elsheria', 1.6e6]]);
const SOURCE_TRIS = new Map([
  ['veil_platform', 182_000],
  ['observatory_arch', 96_000],
  ['crystal_lantern', 41_000],
  ['elsheria', 64_000],
]);

const kb = (n: number | null) => (n === null ? '' : String(Math.round(n / 1000)));

/** The site file each model ships as. */
function shipName(m: ModelFile, settings: Settings): string {
  const id = m.model.replace(/_/g, '-');
  return settings.assets[id]?.file ?? m.model;
}

function planFile(m: ModelFile, settings: Settings): SitePlanFile {
  const asset = m.model.replace(/_/g, '-');
  const choice = settings.assets[asset]?.choice ?? 'auto';
  const models = settings.style === 'models';
  const before = HAND_MADE.has(m.model)
    ? 1.9e6
    : (SMALLER.get(m.model) ?? BEFORE.get(m.model) ?? null);
  let held: string | null = null;
  if (choice === 'never') held = 'set to never ship';
  else if (choice === 'auto' && HAND_MADE.has(m.model))
    held = 'the site file was not made by Maquette';
  else if (choice === 'auto' && models && SMALLER.has(m.model)) held = 'the site file is smaller';
  const tris = SOURCE_TRIS.get(m.model) ?? null;
  return {
    file: `${shipName(m, settings)}.glb`,
    asset,
    assetName: m.assetName,
    change: HAND_MADE.has(m.model) ? 'hand-made' : before === null ? 'new' : 'replaced',
    choice,
    ships: held === null,
    held,
    bytes: m.shippedBytes,
    beforeBytes: before,
    triangles: tris === null ? null : Math.round(tris * 0.6),
    beforeTriangles: before === null || tris === null ? null : Math.round(tris * 0.7),
  };
}

function sizeRow(m: ModelFile, settings: Settings): SizeRow {
  const tris = SOURCE_TRIS.get(m.model) ?? null;
  return {
    asset: m.model.replace(/_/g, '-'),
    name: m.assetName ?? m.model,
    version: m.version ?? 1,
    sourceFile: `models/${m.model}_v${m.version ?? 1}.glb`,
    sourceBytes: m.rawBytes,
    sourceTriangles: tris,
    shippedFile: `web/dist/${shipName(m, settings)}.glb`,
    shippedBytes: m.shippedBytes,
    shippedTriangles: tris === null ? null : Math.round(tris * 0.6),
  };
}

function sum(values: (number | null)[]): number | null {
  return values.every(v => v !== null) ? values.reduce<number>((a, v) => a + (v ?? 0), 0) : null;
}

/** Frostveil's site, and an empty one for the other projects. */
export function createShipHandlers({
  project,
  view,
  state,
}: ShipContext): Record<string, (args: Args) => unknown> {
  const settings = new Map<string, Settings>();
  const targets = new Map<string, SiteTarget>();
  if (state === 'hand-built') {
    settings.set('frostveil', {
      ...DEFAULTS,
      style: 'models',
      textures: 'webp',
      assets: { 'observatory-arch': { choice: 'never' } },
    });
    targets.set('frostveil', {
      models: 'apps/landing/public/veil/models',
      posters: null,
      data: null,
    });
  }

  function shipView(id: string): ShipView {
    const p = project(id);
    if (gameOf(p)) fail('invalid', 'A game project ships with the game Ship plan.');
    const s = settings.get(id) ?? DEFAULTS;
    const target = targets.get(id) ?? LEGACY;
    const models = view(id).models;
    const files = models.map(m => planFile(m, s));
    const rows = models.map(m => sizeRow(m, s));
    const held = files.filter(f => !f.ships).length;
    const checks: ShipCheck[] = [
      { ok: p.siteRepo !== null, text: p.siteRepo ? 'A site repo is set' : 'No site repo is set' },
      { ok: true, text: `The models folder ${target.models}/ is in the site` },
    ];
    if (s.style === 'models') {
      checks.push({ ok: target.data === null, text: 'No scene data, the site places the models' });
      checks.push({ ok: true, text: 'Build scene and Export GLB can export at the own origin' });
    }
    if (held) checks.push({ ok: true, text: `${held} held back, the site keeps its file` });
    const csvRows = rows.map(r =>
      [
        r.asset,
        r.name,
        r.version,
        r.sourceBytes,
        r.sourceTriangles,
        r.shippedBytes,
        r.shippedTriangles,
      ].join(',')
    );
    const mdRows = rows.map(
      r =>
        `| ${r.name} | ${kb(r.sourceBytes)} | ${r.sourceTriangles ?? ''} | ${kb(r.shippedBytes)} | ${r.shippedTriangles ?? ''} |`
    );
    return {
      project: id,
      settings: s,
      plan: {
        target: {
          repo: p.siteRepo,
          ...target,
          configured: targets.has(id),
          modelsExist: true,
          error: null,
        },
        style: s.style,
        textures: s.textures,
        exported: models.length > 0,
        files,
        left: [{ file: 'snow_drift.glb', bytes: 0.72e6, removed: false }],
        checks,
        scriptReady: true,
        originReady: true,
      },
      report: {
        project: id,
        textures: s.textures,
        rows,
        totals: {
          models: rows.length,
          sourceBytes: sum(rows.map(r => r.sourceBytes)),
          shippedBytes: sum(rows.map(r => r.shippedBytes)),
          sourceTriangles: sum(rows.map(r => r.sourceTriangles)),
          shippedTriangles: sum(rows.map(r => r.shippedTriangles)),
        },
        csv: [
          'asset,name,version,source_bytes,source_triangles,shipped_bytes,shipped_triangles',
          ...csvRows,
        ].join('\n'),
        markdown: [
          '| Model | Source KB | Source tris | Shipped KB | Shipped tris |',
          '| --- | ---: | ---: | ---: | ---: |',
          ...mdRows,
        ].join('\n'),
      },
    };
  }

  return {
    ship_get: ({ project: id }) => shipView(String(id)),
    ship_save: ({ project: id, change }) => {
      const pid = String(id);
      shipView(pid);
      const { settings: next, target } = (change ?? {}) as ShipSave;
      const old = settings.get(pid) ?? DEFAULTS;
      const quality = next?.quality ?? old.quality;
      if (quality < 1 || quality > 100) fail('invalid', 'The quality must be 1 to 100.');
      const assets: Record<string, Entry> = { ...old.assets };
      for (const [asset, entry] of Object.entries(next?.assets ?? {})) {
        const file = entry.file?.replace(/\.glb$/, '') || null;
        if (file && !/^[a-z0-9_-]{1,64}$/.test(file))
          fail('invalid', `${file} is not a file name: use a to z, 0 to 9, _ and -.`);
        const choice = entry.choice ?? 'auto';
        if (choice === 'auto' && !file) delete assets[asset];
        else assets[asset] = file ? { choice, file } : { choice };
      }
      settings.set(pid, {
        ...old,
        style: next?.style ?? old.style,
        textures: next?.textures ?? old.textures,
        quality,
        assets,
      });
      if (target) targets.set(pid, target);
      return shipView(pid);
    },
  };
}
