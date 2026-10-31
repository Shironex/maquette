/**
 * The library in mock mode: entries kept in memory and copied in and out
 * of the mock projects the way the core does it. `?library=demo` opens with
 * the entries of `library-fixtures.ts`, `?library=scale` with 214 models
 * (`library-scale-fixtures.ts`); without either the library is empty. The
 * bulk actions live in `create-library-batch-handlers.ts`.
 * Adding, using and updating spend nothing, as in the app.
 */
import type {
  AddToLibrary,
  Asset,
  LibraryAdded,
  LibraryEntry,
  LibraryItem,
  LibraryUpdated,
  LibraryUsed,
  LibraryView,
  ModelPrep,
  PackRecord,
  ProjectManifest,
  SettingChange,
  UpdatePlan,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, slugify, type Args } from './backend-options';
import { LIBRARY_ORIGINS } from './library-fixtures';
import { createAnimationHandlers } from './create-animation-handlers';
import { createLibraryBatchHandlers, seedLibrary } from './create-library-batch-handlers';
import { createPackHandlers } from './create-pack-handlers';

interface LibraryContext {
  projects: () => readonly ProjectManifest[];
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  root: () => string;
  prep: (project: string, asset: string) => ModelPrep | null;
  savePrep: (project: string, asset: string, entry: ModelPrep) => void;
  /** Serves a project file at a library path and back, since mock mode has no disk. */
  alias: (path: string, from: string) => void;
  state: string | null;
  /** Export packs: how fast a model runs, and the pack changed event. */
  pack: { pace: number; emit: (record: PackRecord) => void };
}

const key = (id: string) => id.replace(/-/g, '_');
const ext = (file: string) => file.split('.').pop()?.toLowerCase() ?? 'glb';

function withAsset(p: ProjectManifest, id: string, change: (a: Asset) => Asset): ProjectManifest {
  return { ...p, assets: (p.assets ?? []).map(a => (a.id === id ? change(a) : a)) };
}

export function createLibraryHandlers(ctx: LibraryContext) {
  const state = seedLibrary(ctx.state);
  if (ctx.state === 'demo' || ctx.state === 'scale') {
    for (const [projectId, origins] of Object.entries(LIBRARY_ORIGINS)) {
      const p = ctx.projects().find(x => x.id === projectId);
      if (!p) continue;
      ctx.saveProject({
        ...p,
        assets: (p.assets ?? []).map(a => (origins[a.id] ? { ...a, origin: origins[a.id] } : a)),
      });
    }
    ctx.savePrep('frostveil', 'elsheria', {
      size: { axis: 'height', value: 170, unit: 'cm' },
      fixups: [{ op: 'decimate', triangles: 60_000 }, { op: 'origin-bottom' }],
      skipBake: true,
      padEdges: null,
    });
  }
  const dir = () => `${ctx.root()}/library`;
  const entry = (id: string) =>
    state.entries.find(e => e.id === id) ??
    fail('not-found', `Could not find the library entry ${id}.`);

  function item(e: LibraryEntry): LibraryItem {
    const latest = e.versions.at(-1)?.version ?? 0;
    const uses = ctx.projects().flatMap(p =>
      (p.assets ?? [])
        .filter(a => a.origin?.entry === e.id)
        .map(a => ({
          project: p.id,
          projectName: p.name,
          target: p.target ?? 'website',
          asset: a.id,
          assetName: a.name,
          version: a.origin!.version,
          update: a.origin!.version < latest,
        }))
    );
    const credits = (n: number) => e.versions.find(v => v.version === n)?.credits ?? 0;
    const own = (u: { project: string; asset: string }) =>
      e.versions.some(v => v.from.project === u.project && v.from.asset === u.asset);
    const file = e.versions.at(-1)?.file ?? '';
    return {
      entry: e,
      dir: `${dir()}/${e.id}`,
      format: ext(file) === 'obj' ? 'obj' : 'glb',
      uses,
      spent: e.versions.reduce((sum, v) => sum + (v.credits ?? 0), 0),
      saved: uses.filter(u => !own(u)).reduce((sum, u) => sum + credits(u.version), 0),
    };
  }

  const view = (): LibraryView => {
    const known = new Set(state.entries.map(e => e.id));
    return {
      dir: dir(),
      items: state.entries.map(item),
      collections: state.collections.map(c => ({
        ...c,
        entries: c.entries.filter(id => known.has(id)),
      })),
    };
  };

  function note(p: ProjectManifest, file: string, asset: string): string | null {
    const format = ext(file) === 'obj' ? 'obj' : 'glb';
    if ((p.target ?? 'website') === 'website') {
      if (format !== 'glb') {
        fail(
          'invalid',
          `${p.name} builds its scene from .glb models, and this one is .${format}. Use it in a game project, or add a .glb of it to the library.`
        );
      }
      return null;
    }
    const wanted = p.game?.format ?? 'glb';
    const target = `${p.game?.modelsDir ?? 'assets/models'}/${key(asset)}.${wanted}`;
    return format === wanted
      ? `The next run exports it to ${target} in the game.`
      : `This game takes .${wanted} and the model is .${format}, so the next run's Game export writes it as .${wanted} to ${target}. Nothing is converted now.`;
  }

  function add({ request }: Args): LibraryAdded {
    const r = request as AddToLibrary;
    const p = ctx.project(r.project);
    const asset = (p.assets ?? []).find(a => a.id === r.asset);
    if (!asset) fail('not-found', `Could not find the asset ${r.asset}.`);
    const latest = asset.versions.at(-1);
    if (!latest) {
      fail(
        'invalid',
        `${asset.name} has no model yet. Accept or build one first, then add it to the library.`
      );
    }
    const hash = `${r.project}/${latest.file}`;
    const now = clock.now().toISOString();
    const into = r.into ? entry(r.into) : undefined;
    let found =
      into ??
      state.entries.find(e => e.id === asset.origin?.entry) ??
      state.entries.find(e =>
        e.versions.some(v => v.from.project === r.project && v.from.asset === asset.id)
      );
    if (!found) {
      const base = slugify(r.name ?? asset.name);
      let id = base;
      for (let n = 2; state.entries.some(e => e.id === id); n++) id = `${base}-${n}`;
      found = {
        version: 1,
        id,
        name: r.name ?? asset.name,
        kind: asset.kind,
        tags: [],
        refs: [],
        versions: [],
        createdAt: now,
        updatedAt: now,
      };
      state.entries = [...state.entries, found];
    }
    const tags = r.tags
      ? [...new Set(r.tags.map(t => t.trim().toLowerCase()).filter(Boolean))]
      : found.tags;
    const known = found.versions.find(v => v.hash === hash);
    const number = known?.version ?? found.versions.length + 1;
    const thumbnail = asset.thumbnail
      ? `thumbs/${key(found.id)}_v${number}.${ext(asset.thumbnail)}`
      : null;
    if (thumbnail && asset.thumbnail) {
      ctx.alias(
        `${dir()}/${found.id}/${thumbnail}`,
        `${ctx.root()}/projects/${p.id}/${asset.thumbnail}`
      );
    }
    const next: LibraryEntry = {
      ...found,
      name: r.name ?? found.name,
      tags,
      updatedAt: now,
      versions: known
        ? found.versions
        : [
            ...found.versions,
            {
              version: number,
              file: `models/${key(found.id)}_v${number}.${ext(latest.file)}`,
              createdAt: now,
              stats: latest.stats,
              source: 'tripo',
              credits: null,
              hash,
              from: { project: p.id, asset: asset.id, version: latest.version },
              thumbnail,
              prep: ctx.prep(p.id, asset.id),
              materials: null,
            },
          ],
    };
    state.entries = state.entries.map(e => (e.id === next.id ? next : e));
    const project = ctx.saveProject(
      withAsset(p, asset.id, a => ({ ...a, origin: { entry: next.id, version: number } }))
    );
    return { entry: next, version: number, added: !known, project };
  }

  function use({ entry: id, version, project: projectId }: Args): LibraryUsed {
    const e = entry(String(id));
    const chosen = version ? e.versions.find(v => v.version === version) : e.versions.at(-1);
    if (!chosen) fail('not-found', `Could not find that version of ${e.name}.`);
    const p = ctx.project(String(projectId));
    const taken = (name: string) =>
      (p.assets ?? []).some(a => a.id === slugify(name) || a.name === name);
    let name = e.name;
    for (let n = 2; taken(name); n++) name = `${e.name} ${n}`;
    const assetId = slugify(name);
    const said = note(p, chosen.file, assetId);
    const now = clock.now().toISOString();
    const thumbnail = chosen.thumbnail
      ? `thumbs/${key(assetId)}_v1.${ext(chosen.thumbnail)}`
      : null;
    if (thumbnail && chosen.thumbnail) {
      ctx.alias(
        `${ctx.root()}/projects/${p.id}/${thumbnail}`,
        `${dir()}/${e.id}/${chosen.thumbnail}`
      );
    }
    const asset: Asset = {
      id: assetId,
      name,
      kind: e.kind,
      stage: 'blender',
      status: 'done',
      versions: [
        {
          version: 1,
          file: `models/${key(assetId)}_v1.${ext(chosen.file)}`,
          createdAt: now,
          stats: chosen.stats,
        },
      ],
      thumbnail,
      createdAt: now,
      updatedAt: now,
      origin: { entry: e.id, version: chosen.version },
    };
    const project = ctx.saveProject({ ...p, assets: [...(p.assets ?? []), asset] });
    if (chosen.prep) ctx.savePrep(p.id, assetId, chosen.prep);
    return { asset, project, note: said, settings: Boolean(chosen.prep) };
  }

  function plan({ project: projectId, asset: assetId }: Args): UpdatePlan {
    const p = ctx.project(String(projectId));
    const asset = (p.assets ?? []).find(a => a.id === assetId);
    if (!asset) fail('not-found', `Could not find the asset ${String(assetId)}.`);
    if (!asset.origin) fail('invalid', `${asset.name} did not come from the library.`);
    const e = entry(asset.origin.entry);
    const newest = e.versions.at(-1)!;
    if (newest.version <= asset.origin.version) {
      fail(
        'invalid',
        `${asset.name} is already on the library's newest version, v${asset.origin.version}.`
      );
    }
    const prep = ctx.prep(p.id, asset.id);
    const kept: string[] = prep?.size
      ? [`Real size, ${prep.size.value} ${prep.size.unit} high`]
      : [];
    const dropped: SettingChange[] = [];
    for (const f of prep?.fixups ?? []) {
      const tris = newest.stats?.triangles ?? Infinity;
      if (f.op === 'decimate' && tris <= f.triangles) {
        dropped.push({
          what: `Decimate to ${Math.round(f.triangles / 1000)}k`,
          why: `the new model has ${Math.round(tris / 1000)}k triangles, already at or under ${Math.round(f.triangles / 1000)}k`,
        });
      } else {
        kept.push(f.op === 'origin-bottom' ? 'Origin to bottom centre' : f.op);
      }
    }
    if (prep?.skipBake) kept.push('Skips the bake');
    const nextVersion = Math.max(0, ...asset.versions.map(v => v.version)) + 1;
    return {
      project: p.id,
      asset: asset.id,
      name: asset.name,
      entry: e.id,
      from: asset.origin.version,
      to: newest.version,
      nextVersion,
      kept,
      dropped,
      note: note(p, newest.file, asset.id),
    };
  }

  function update(args: Args): LibraryUpdated {
    const found = plan(args);
    const p = ctx.project(found.project);
    const e = entry(found.entry);
    const newest = e.versions.at(-1)!;
    const now = clock.now().toISOString();
    const thumbnail = newest.thumbnail
      ? `thumbs/${key(found.asset)}_v${found.nextVersion}.${ext(newest.thumbnail)}`
      : null;
    if (thumbnail && newest.thumbnail) {
      ctx.alias(
        `${ctx.root()}/projects/${p.id}/${thumbnail}`,
        `${dir()}/${e.id}/${newest.thumbnail}`
      );
    }
    const project = ctx.saveProject(
      withAsset(p, found.asset, a => ({
        ...a,
        versions: [
          ...a.versions,
          {
            version: found.nextVersion,
            file: `models/${key(a.id)}_v${found.nextVersion}.${ext(newest.file)}`,
            createdAt: now,
            stats: newest.stats,
          },
        ],
        thumbnail: thumbnail ?? a.thumbnail,
        origin: { entry: e.id, version: newest.version },
        updatedAt: now,
      }))
    );
    const prep = ctx.prep(p.id, found.asset);
    if (prep && found.dropped.length > 0) {
      ctx.savePrep(p.id, found.asset, {
        ...prep,
        fixups: prep.fixups.filter(f => f.op !== 'decimate'),
      });
    }
    const asset = (project.assets ?? []).find(a => a.id === found.asset)!;
    return { plan: found, asset, project };
  }

  const animations = createAnimationHandlers({
    project: ctx.project,
    saveProject: ctx.saveProject,
    alias: ctx.alias,
    root: ctx.root,
    state: ctx.state,
  });

  const batch = createLibraryBatchHandlers({
    state,
    projects: ctx.projects,
    project: ctx.project,
    saveProject: ctx.saveProject,
  });

  const packs = createPackHandlers({
    library: view,
    project: ctx.project,
    root: ctx.root,
    emit: ctx.pack.emit,
    pace: ctx.pack.pace,
    state: ctx.state,
  });

  return {
    handlers: {
      ...animations.handlers,
      ...batch,
      ...packs.handlers,
      library_get: () => view(),
      library_add: add,
      library_use: use,
      library_update_plan: plan,
      library_update: update,
    } as Record<string, (args: Args) => unknown>,
  };
}
