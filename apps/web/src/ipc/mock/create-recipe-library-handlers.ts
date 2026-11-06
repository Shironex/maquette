/**
 * The recipe library in mock mode: library recipes kept in memory with
 * their revisions, with the core's checks. A save always adds a revision
 * and never overwrites one; a save, a removal or a plan read against an
 * entry or recipe that moved since is refused (`moved`); a recipe that
 * holds what the newest revision holds has nothing to save. Use and import
 * copy a recipe into a project under a fresh id, opening prop blanks the
 * project lacks and noting a base clip that is missing or has other keys.
 * Mock mode has no file dialogs: export answers a made up path and both
 * imports read the same canned file (see recipe-library-fixtures.ts).
 * Starters (recipe-starters.ts) list, are used and are copied to the
 * library as the core does, never written there by a list.
 */
import type {
  AnimationClip,
  ExportSource,
  LibraryRecipeDoc,
  LibraryRecipeSummary,
  ProjectManifest,
  RecipeDoc,
  RecipeFile,
  RecipeSave,
  RecipeSavePlan,
  UseRecipe,
} from '../client';
import { events } from '../generated/bindings';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { HOME } from './fixtures';
import { slugOf } from './plan-store';
import { recipeChanges } from './recipe-changes';
import { isMixamo, matchBase, versionOf } from './recipe-files';
import {
  IMPORT_FILE,
  demoLibrary,
  importedFile,
  revisionOf,
  storedEntry,
  type StoredEntry,
} from './recipe-library-fixtures';
import { currentSaveAsks } from './recipe-save-asks';
import { STARTER_PREFIX, starterFile, starterList } from './recipe-starters';

interface LibraryContext {
  clips: () => readonly AnimationClip[];
  project: (id: string) => ProjectManifest;
  /** A project's recipes, the demo ones seeded on the first read. */
  recipes: (project: string) => Map<string, RecipeFile>;
  /** A project recipe as `recipe_get` reads it. */
  doc: (project: string, id: string, file: RecipeFile) => RecipeDoc;
  /** A free recipe id in `project` for `name`. */
  freeId: (project: string, name: string) => string;
  state: string | null;
}

const changed = () => void events.recipeLibraryChanged.emit().catch(() => {});

export function createRecipeLibraryHandlers(ctx: LibraryContext) {
  let library: Map<string, StoredEntry> | null = null;
  const entries = () => {
    if (!library)
      library =
        ctx.state === 'demo'
          ? demoLibrary(ctx.clips(), ctx.recipes('frostveil').get('reach-for-the-lantern'))
          : new Map();
    return library;
  };
  const entryOf = (id: unknown) => {
    const found = entries().get(String(id));
    if (!found) fail('not_found', `Not found: the library recipe ${String(id)}`);
    return found;
  };
  const latestOf = (e: StoredEntry) => Math.max(0, ...e.entry.revisions.map(r => r.revision));
  const fileAt = (e: StoredEntry, revision: number | null) => {
    const at = revision ?? latestOf(e);
    const file = e.files.get(at);
    if (!file || !e.entry.revisions.some(r => r.revision === at))
      fail('not_found', `Not found: revision r${at} of the library recipe ${e.entry.id}`);
    return { at, file };
  };
  const checkExpected = (e: StoredEntry, expected: unknown) => {
    const now = versionOf(e.entry);
    if (expected != null && expected !== now)
      fail(
        'moved',
        `The library recipe ${e.entry.id} changed since it was read (its version is now ${now}). Read it again.`
      );
  };
  const recipeOf = (project: unknown, recipe: unknown) => {
    const file = ctx.recipes(String(project)).get(String(recipe));
    if (!file) fail('not_found', `Not found: the recipe ${String(recipe)}`);
    return file;
  };
  const freeEntryId = (name: string) => {
    const base = /[a-z0-9]/i.test(name) ? slugOf(name) : 'recipe';
    let id = base;
    for (let n = 2; entries().has(id); n += 1) id = `${base}-${n}`;
    return id;
  };
  const summary = (e: StoredEntry): LibraryRecipeSummary => {
    const latest = latestOf(e);
    const newest = e.files.get(latest);
    return {
      id: e.entry.id,
      uid: e.entry.uid,
      name: e.entry.name,
      version: versionOf(e.entry),
      latest,
      revisions: e.entry.revisions,
      base: newest?.base.name ?? '',
      frames: newest?.base.frames ?? 0,
      mixamo: newest ? isMixamo(ctx.clips(), newest.base) : false,
      layers: newest?.layers.length ?? 0,
      blanks: newest?.blanks.length ?? 0,
      updatedAt: e.entry.updatedAt,
      problem: newest ? null : `The newest revision of ${e.entry.name} could not be read.`,
    };
  };

  /** Copies `file` into `project` as a new recipe, with what to know about it. */
  const land = (project: string, file: RecipeFile, notes: string[]) => {
    const manifest = ctx.project(project);
    const blanks = file.blanks.map(b => {
      if (b.kind !== 'prop' || !b.prop || manifest.assets?.some(a => a.id === b.prop)) return b;
      notes.push(
        `The blank ${b.id} named ${b.prop}, which ${manifest.name} does not have, so it is open. Fill it with a prop of this project.`
      );
      return { ...b, prop: null };
    });
    const wanted = slugOf(file.name);
    const id = ctx.freeId(project, file.name);
    if (id !== wanted)
      notes.push(`The project has a recipe ${wanted} already, so this one is ${id}.`);
    const now = clock.now().toISOString();
    const stored: RecipeFile = { ...file, blanks, created: now, edited: now };
    ctx.recipes(project).set(id, stored);
    const doc = ctx.doc(project, id, stored);
    if (doc.base.warning) notes.push(doc.base.warning);
    return { doc, notes };
  };

  const planSave = (project: string, recipe: string, entry: string | null): RecipeSavePlan => {
    const file = recipeOf(project, recipe);
    const plan: RecipeSavePlan = {
      project,
      recipe,
      recipeName: file.name,
      recipeVersion: versionOf(file),
      entry: null,
      entryName: file.name,
      entryVersion: null,
      latest: null,
      revision: 1,
      changes: recipeChanges(null, file),
      same: false,
    };
    if (!entry) return plan;
    const e = entryOf(entry);
    const newest = fileAt(e, null);
    const changes = recipeChanges(newest.file, file);
    return {
      ...plan,
      entry: e.entry.id,
      entryName: e.entry.name,
      entryVersion: versionOf(e.entry),
      latest: newest.at,
      revision: Math.max(e.entry.lastRevision, newest.at) + 1,
      changes,
      same: changes.length === 0,
    };
  };

  const save = ({ project, recipe, entry, expected, recipeExpected }: RecipeSave) => {
    const file = recipeOf(project, recipe);
    if (recipeExpected != null && recipeExpected !== versionOf(file))
      fail('moved', `The recipe ${recipe} changed since the save was planned. Look at it again.`);
    const at = clock.now().toISOString();
    const source = (n: number) => revisionOf(n, at, { project, recipe });
    let stored: StoredEntry;
    if (entry) {
      stored = entryOf(entry);
      if (expected == null)
        fail(
          'invalid',
          `Saving a revision of ${stored.entry.name} needs the version its confirm read.`
        );
      checkExpected(stored, expected);
      const newest = fileAt(stored, null);
      if (recipeChanges(newest.file, file).length === 0)
        fail(
          'invalid',
          `${file.name} holds what r${newest.at} of ${stored.entry.name} holds already, so there is nothing to save.`
        );
      const next = Math.max(stored.entry.lastRevision, newest.at) + 1;
      stored.files.set(next, { ...file, revision: next, from: null });
      stored.entry = {
        ...stored.entry,
        revisions: [...stored.entry.revisions, source(next)],
        lastRevision: next,
        updatedAt: at,
      };
    } else {
      const id = freeEntryId(file.name);
      stored = storedEntry(id, file.name, [[file, source(1)]]);
      entries().set(id, stored);
    }
    const revision = stored.entry.lastRevision;
    ctx
      .recipes(project)
      .set(recipe, { ...file, from: { entry: stored.entry.id, uid: stored.entry.uid, revision } });
    currentSaveAsks()?.close(project, recipe);
    changed();
    return {
      entry: stored.entry.id,
      revision,
      entryVersion: versionOf(stored.entry),
      linked: true,
    };
  };

  return {
    recipe_library_list: () =>
      [...entries().values()].map(summary).sort((a, b) => a.name.localeCompare(b.name)),
    recipe_library_get: ({ entry, revision }: Args): LibraryRecipeDoc => {
      const e = entryOf(entry);
      const { at, file } = fileAt(e, (revision as number | null) ?? null);
      return {
        entry: e.entry,
        version: versionOf(e.entry),
        revision: at,
        recipe: file,
        base: matchBase(ctx.clips(), file.base)[0],
        mixamo: isMixamo(ctx.clips(), file.base),
      };
    },
    recipe_starter_list: () => starterList(ctx.clips()),
    recipe_library_copy_starter: ({ id }: Args) => {
      const file = starterFile(String(id), ctx.clips());
      if (!file) fail('not_found', `Not found: the starter recipe ${String(id)}`);
      const notes: string[] = [];
      const wanted = slugOf(file.name);
      const entry = freeEntryId(file.name);
      if (entry !== wanted)
        notes.push(`The library has a recipe ${wanted} already, so this one is ${entry}.`);
      const base = matchBase(ctx.clips(), file.base)[0];
      if (base.warning) notes.push(base.warning);
      const at = clock.now().toISOString();
      const source = revisionOf(1, at, { imported: `Starter: ${file.name}` });
      entries().set(entry, storedEntry(entry, file.name, [[file, source]]));
      changed();
      return { entry, name: file.name, notes };
    },
    recipe_library_use: ({ request }: Args) => {
      const { project, entry, revision, name } = request as UseRecipe;
      if (entry.startsWith(STARTER_PREFIX)) {
        if (revision != null)
          fail('invalid', 'A starter recipe has no revisions. Leave revision out.');
        const file = starterFile(entry, ctx.clips());
        if (!file) fail('not_found', `Not found: the starter recipe ${entry}`);
        return land(project, { ...file, name: name?.trim() || file.name }, []);
      }
      const e = entryOf(entry);
      const { at, file } = fileAt(e, revision);
      const named = name?.trim() || file.name;
      return land(
        project,
        { ...file, name: named, from: { entry: e.entry.id, uid: e.entry.uid, revision: at } },
        []
      );
    },
    recipe_library_plan_save: ({ project, recipe, entry }: Args) =>
      planSave(String(project), String(recipe), (entry as string | null) ?? null),
    recipe_library_compare: ({ project, recipe }: Args) => {
      const file = recipeOf(project, recipe);
      if (!file.from) fail('invalid', `${file.name} came from no library recipe.`);
      const e = entryOf(file.from.entry);
      if (e.entry.uid !== file.from.uid)
        fail('not-found', `the library recipe ${file.name} came from was not found`);
      const newest = fileAt(e, null);
      return {
        entry: e.entry.id,
        entryName: e.entry.name,
        revision: file.from.revision,
        latest: newest.at,
        changes: recipeChanges(file, newest.file),
      };
    },
    recipe_library_save: ({ save: request }: Args) => save(request as RecipeSave),
    recipe_library_remove_revision: ({ entry, revision, expected }: Args) => {
      const e = entryOf(entry);
      checkExpected(e, expected);
      if (!e.entry.revisions.some(r => r.revision === revision))
        fail(
          'not_found',
          `Not found: revision r${String(revision)} of the library recipe ${e.entry.id}`
        );
      if (e.entry.revisions.length === 1)
        fail(
          'invalid',
          `r${String(revision)} is the only revision of ${e.entry.name}. Remove the whole recipe instead.`
        );
      e.files.delete(revision as number);
      e.entry = {
        ...e.entry,
        revisions: e.entry.revisions.filter(r => r.revision !== revision),
        updatedAt: clock.now().toISOString(),
      };
      changed();
      return e.entry;
    },
    recipe_library_remove: ({ entry, expected }: Args) => {
      const e = entryOf(entry);
      checkExpected(e, expected);
      entries().delete(e.entry.id);
      changed();
      return `removed/${e.entry.id}-${clock.nowMs()}`;
    },
    recipe_export: ({ source, name }: Args) => {
      const { project, recipe, entry, revision } = source as ExportSource;
      if (entry) fileAt(entryOf(entry), revision);
      else recipeOf(project, recipe);
      const file = `${slugOf(String(name)) || 'recipe'}.recipe.json`;
      return { path: `${HOME}/Downloads/${file}`, file };
    },
    recipe_import_to_project: ({ project }: Args) =>
      land(String(project), importedFile(ctx.clips()), []),
    recipe_import_to_library: () => {
      const file = importedFile(ctx.clips());
      const notes: string[] = [];
      const wanted = slugOf(file.name);
      const id = freeEntryId(file.name);
      if (id !== wanted)
        notes.push(`The library has a recipe ${wanted} already, so this one is ${id}.`);
      const base = matchBase(ctx.clips(), file.base)[0];
      if (base.warning) notes.push(base.warning);
      const at = clock.now().toISOString();
      entries().set(
        id,
        storedEntry(id, file.name, [[file, revisionOf(1, at, { imported: IMPORT_FILE })]])
      );
      changed();
      return { entry: id, name: file.name, notes };
    },
  } as Record<string, (args: Args) => unknown>;
}
