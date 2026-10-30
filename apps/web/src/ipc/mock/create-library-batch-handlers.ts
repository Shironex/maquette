/**
 * The library's bulk actions in mock mode, as the core does them: tags on
 * many entries, rename, collections, remove and restore (a removed entry
 * waits in memory, a merged one with what undoes it), look-alikes, merge,
 * and the plan for adding a whole project.
 */
import type {
  CollectionChange,
  LibraryCollection,
  LibraryEntry,
  LibraryMerge,
  LibraryMerged,
  LibraryRestored,
  LookAlikeSettings,
  MergeEntries,
  ProjectManifest,
  TagEntries,
} from '../client';
import { clock } from '@/lib/clock';
import { fail, slugify, type Args } from './backend-options';
import { scaleLibrary } from './library-scale-fixtures';
import { libraryEntries } from './library-fixtures';
import { addManyPlan, lookAlikes, pairId } from './library-looks';

export interface Removed {
  entry: LibraryEntry;
  /** Set for a merged entry: undoing puts the versions and origins back. */
  merge?: LibraryMerge;
  /** The kept entry before the merge, and the origins it changed. */
  before?: LibraryEntry;
  origins?: { project: string; asset: string; version: number }[];
}

export interface LibraryState {
  entries: LibraryEntry[];
  collections: LibraryCollection[];
  removed: Map<string, Removed>;
  decided: Set<string>;
  settings: LookAlikeSettings;
}

const DEFAULTS: LookAlikeSettings = { threshold: 0.9, checkOnAdd: true, copyTags: true };

export function seedLibrary(state: string | null): LibraryState {
  const scale = state === 'scale' ? scaleLibrary() : null;
  return {
    entries: scale?.entries ?? (state === 'demo' ? libraryEntries() : []),
    collections: scale?.collections ?? [],
    removed: new Map(scale?.merged.map(m => [m.entry.id, m]) ?? []),
    decided: new Set(),
    settings: DEFAULTS,
  };
}

interface BatchContext {
  state: LibraryState;
  projects: () => readonly ProjectManifest[];
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
}

const cleanTags = (tags: readonly string[]) => [
  ...new Set(tags.map(t => t.trim().toLowerCase()).filter(Boolean)),
];

export function createLibraryBatchHandlers(ctx: BatchContext) {
  const { state } = ctx;
  const now = () => clock.now().toISOString();
  const entry = (id: string) =>
    state.entries.find(e => e.id === id) ??
    fail('not-found', `Could not find the library entry ${id}.`);
  const save = (next: LibraryEntry) => {
    state.entries = state.entries.map(e => (e.id === next.id ? next : e));
    return next;
  };
  const sorted = () =>
    [...state.collections].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
  const collection = (id: string) =>
    state.collections.find(c => c.id === id) ??
    fail('not-found', `Could not find the collection ${id}.`);
  /** Points every asset that came from `from` at `to`, versions mapped; answers the projects changed. */
  const repoint = (from: string, to: string, version: (old: number) => number) =>
    ctx.projects().flatMap(p => {
      if (!(p.assets ?? []).some(a => a.origin?.entry === from)) return [];
      return [
        ctx.saveProject({
          ...p,
          assets: (p.assets ?? []).map(a =>
            a.origin?.entry === from
              ? { ...a, origin: { entry: to, version: version(a.origin.version) } }
              : a
          ),
        }),
      ];
    });

  function tag({ request }: Args): LibraryEntry[] {
    const r = request as TagEntries;
    const found = r.entries.map(entry);
    const add = cleanTags(r.add);
    const remove = new Set(cleanTags(r.remove));
    const next = found.map(e => ({
      ...e,
      tags: cleanTags([...e.tags.filter(t => !remove.has(t)), ...add]),
    }));
    if (next.some(e => e.tags.length > 16)) fail('invalid', 'Give a model 16 tags or fewer.');
    return next.map(e => save({ ...e, updatedAt: now() }));
  }

  function rename({ entry: id, name }: Args): LibraryEntry {
    const clean = String(name).trim();
    if (!clean) fail('invalid', 'Give the model a name.');
    return save({ ...entry(String(id)), name: clean, updatedAt: now() });
  }

  function changeCollection({ change }: Args): LibraryCollection[] {
    const c = change as CollectionChange;
    const taken = (name: string, except?: string) =>
      state.collections.some(x => x.id !== except && x.name.toLowerCase() === name.toLowerCase());
    if (c.op === 'create') {
      const name = c.name.trim();
      if (!name) fail('invalid', 'Give the collection a name.');
      if (taken(name)) fail('invalid', `There is a collection called ${name} already.`);
      c.entries.forEach(entry);
      let id = slugify(name);
      for (let n = 2; state.collections.some(x => x.id === id); n++) id = `${slugify(name)}-${n}`;
      state.collections = [
        ...state.collections,
        { id, name, entries: [...new Set(c.entries)], createdAt: now() },
      ];
    } else if (c.op === 'delete') {
      collection(c.collection);
      state.collections = state.collections.filter(x => x.id !== c.collection);
    } else {
      const found = collection(c.collection);
      let next = found;
      if (c.op === 'add') {
        c.entries.forEach(entry);
        next = { ...found, entries: [...new Set([...found.entries, ...c.entries])] };
      } else if (c.op === 'remove') {
        next = { ...found, entries: found.entries.filter(id => !c.entries.includes(id)) };
      } else {
        if (taken(c.name, found.id))
          fail('invalid', `There is a collection called ${c.name} already.`);
        next = { ...found, name: c.name.trim() };
      }
      state.collections = state.collections.map(x => (x.id === next.id ? next : x));
    }
    return sorted();
  }

  function remove({ entries }: Args): string[] {
    const ids = entries as string[];
    const found = ids.map(entry);
    for (const e of found) state.removed.set(e.id, { entry: e });
    state.entries = state.entries.filter(e => !ids.includes(e.id));
    return ids;
  }

  function restore({ entries }: Args): LibraryRestored {
    const ids = entries as string[];
    const projects: ProjectManifest[] = [];
    for (const id of ids) {
      const r = state.removed.get(id) ?? fail('not-found', `Nothing removed is called ${id}.`);
      if (state.entries.some(e => e.id === id))
        fail('invalid', `${r.entry.name} is in the library again.`);
      if (r.merge && r.before) {
        save(r.before);
        for (const o of r.origins ?? []) {
          const p = ctx.project(o.project);
          projects.push(
            ctx.saveProject({
              ...p,
              assets: (p.assets ?? []).map(a =>
                a.id === o.asset ? { ...a, origin: { entry: id, version: o.version } } : a
              ),
            })
          );
        }
      }
      state.entries = [...state.entries, r.entry];
      state.removed.delete(id);
    }
    return { entries: ids, projects };
  }

  function merge({ request }: Args): LibraryMerged {
    const r = request as MergeEntries;
    if (r.keep === r.merge) fail('invalid', 'Pick two different models to merge.');
    const keep = entry(r.keep);
    const gone = entry(r.merge);
    const map = new Map<number, number>();
    const versions = [...keep.versions];
    for (const v of gone.versions) {
      const same = versions.find(k => k.hash === v.hash);
      if (same) {
        map.set(v.version, same.version);
        continue;
      }
      const n = Math.max(0, ...versions.map(k => k.version)) + 1;
      versions.push({ ...v, version: n, file: `models/${keep.id.replace(/-/g, '_')}_v${n}.glb` });
      map.set(v.version, n);
    }
    const origins = ctx
      .projects()
      .flatMap(p =>
        (p.assets ?? [])
          .filter(a => a.origin?.entry === gone.id)
          .map(a => ({ project: p.id, asset: a.id, version: a.origin!.version }))
      );
    const tags = r.copyTags ? cleanTags([...keep.tags, ...gone.tags]).slice(0, 16) : keep.tags;
    const next = save({ ...keep, versions, tags, updatedAt: now() });
    const projects = repoint(gone.id, keep.id, v => map.get(v) ?? v);
    const record: LibraryMerge = {
      merged: gone.id,
      mergedName: gone.name,
      into: keep.id,
      intoName: keep.name,
      versions: [...new Set(map.values())],
      at: now(),
    };
    state.entries = state.entries.filter(e => e.id !== gone.id);
    state.removed.set(gone.id, { entry: gone, merge: record, before: keep, origins });
    return { entry: next, merge: record, projects };
  }

  return {
    library_tag: tag,
    library_rename: rename,
    library_collection: changeCollection,
    library_remove: remove,
    library_restore: restore,
    library_merge: merge,
    library_look_alikes: () => lookAlikes(state),
    library_decide: ({ a, b }: Args) => {
      state.decided.add(pairId(String(a), String(b)));
      return null;
    },
    library_look_settings: ({ settings }: Args) => {
      const s = settings as LookAlikeSettings;
      state.settings = { ...s, threshold: Math.min(0.99, Math.max(0.5, s.threshold ?? 0.9)) };
      return state.settings;
    },
    library_add_many_plan: ({ project }: Args) => addManyPlan(state, ctx.project(String(project))),
  } as Record<string, (args: Args) => unknown>;
}
