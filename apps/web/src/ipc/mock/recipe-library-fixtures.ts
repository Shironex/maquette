/**
 * The mock recipe library (`?library=demo`): Reach for the lantern, saved
 * from Frostveil three times (its project recipe came from r1, so the
 * Recipes tab says the library has r3), Wave hello, imported from a file,
 * and Guard and look on a Mixamo sword clip. Plus the file the mock's
 * import commands bring in, since mock mode has no file picker: its base
 * clip has other keys and its prop blank names a prop no project has.
 */
import type {
  AnimationClip,
  LibraryRecipeEntry,
  LibraryRecipeRevision,
  RecipeFile,
} from '../client';
import { clock } from '@/lib/clock';
import { baseOf, layer, newRecipe } from './recipe-files';

const DAY = 86_400_000;

/** A library recipe and its revisions' files, by revision. */
export interface StoredEntry {
  entry: LibraryRecipeEntry;
  files: Map<number, RecipeFile>;
}

/** The file the mock's import commands read. */
export const IMPORT_FILE = 'sit-and-wave.recipe.json';

const ago = (days: number) => new Date(clock.nowMs() - days * DAY).toISOString();

/** A revision line: saved from a project's recipe, or imported from a file. */
export function revisionOf(
  revision: number,
  savedAt: string,
  from: { project: string; recipe: string } | { imported: string }
): LibraryRecipeRevision {
  return {
    revision,
    file: `r${revision}.recipe.json`,
    savedAt,
    project: 'project' in from ? from.project : null,
    recipe: 'recipe' in from ? from.recipe : null,
    imported: 'imported' in from ? from.imported : null,
  };
}

/** An entry whose revisions are `files`, oldest first, each with where it came from. */
export function storedEntry(
  id: string,
  name: string,
  revisions: readonly [RecipeFile, LibraryRecipeRevision][]
): StoredEntry {
  const files = new Map<number, RecipeFile>();
  for (const [file, rev] of revisions)
    files.set(rev.revision, { ...file, revision: rev.revision, from: null });
  const first = revisions[0]?.[1].savedAt ?? clock.now().toISOString();
  const last = revisions[revisions.length - 1]?.[1];
  return {
    entry: {
      version: 1,
      id,
      name,
      createdAt: first,
      updatedAt: last?.savedAt ?? first,
      lastRevision: last?.revision ?? 0,
      revisions: revisions.map(([, rev]) => rev),
    },
    files,
  };
}

const named = (clips: readonly AnimationClip[], name: string, or: AnimationClip | undefined) =>
  clips.find(c => c.name === name) ?? or;

/** The demo entries; `reach` is Frostveil's lantern recipe as it is now (r1). */
export function demoLibrary(
  clips: readonly AnimationClip[],
  reach: RecipeFile | undefined
): Map<string, StoredEntry> {
  const out = new Map<string, StoredEntry>();
  const from = { project: 'frostveil', recipe: 'reach-for-the-lantern' };
  if (reach) {
    const r2: RecipeFile = {
      ...reach,
      layers: [
        ...reach.layers,
        layer('l-4', 'nudge', 'spine', { start: 0.1, end: 0.4, rotation: [8, 0, 0] }),
      ],
    };
    const r3: RecipeFile = {
      ...r2,
      layers: r2.layers.map(l =>
        l.id === 'l-3' ? { ...l, settings: { ...l.settings, cycles: 3 } } : l
      ),
    };
    out.set(
      'reach-for-the-lantern',
      storedEntry('reach-for-the-lantern', reach.name, [
        [reach, revisionOf(1, ago(8), from)],
        [r2, revisionOf(2, ago(4), from)],
        [r3, revisionOf(3, ago(1), from)],
      ])
    );
  }
  const walk = clips.find(c => c.rig.kind !== 'mixamo') ?? clips[0];
  if (walk) {
    const wave = newRecipe('Wave hello', baseOf(walk));
    wave.layers = [
      layer('l-1', 'wave', 'rightLowerArm', { start: 0.2, end: 0.7, cycles: 3, amount: 30 }),
    ];
    out.set(
      'wave-hello',
      storedEntry('wave-hello', wave.name, [
        [wave, revisionOf(1, ago(6), { imported: 'wave-hello.recipe.json' })],
      ])
    );
  }
  const idle = named(clips, 'Sword And Shield Idle', clips[1]);
  if (idle) {
    const guard = newRecipe('Guard and look', baseOf(idle));
    guard.blanks = [
      { id: 'door', label: 'The door it watches', kind: 'point', prop: null, point: [1, 1.6, 2] },
    ];
    guard.layers = [
      layer('l-1', 'look-at', 'head', { start: 0.3, end: 0.9, weight: 0.7, target: 'door' }),
      layer('l-2', 'nudge', 'chest', { start: 0, end: 1, rotation: [0, 12, 0] }),
    ];
    out.set(
      'guard-and-look',
      storedEntry('guard-and-look', guard.name, [
        [guard, revisionOf(1, ago(3), { project: 'frostveil', recipe: 'guard-and-look' })],
      ])
    );
  }
  return out;
}

/**
 * The recipe file the mock's import commands read: a base clip the library
 * has with other keys (or lacks) and a prop blank no project has.
 */
export function importedFile(clips: readonly AnimationClip[]): RecipeFile {
  const clip = named(clips, 'Sword And Shield Idle', clips[0]);
  const base = clip
    ? { ...baseOf(clip), hash: 'keys-from-another-computer' }
    : { name: 'Sitting Idle', frames: 90, length: 3, hash: 'sitting', mixamo: false };
  const file = newRecipe('Sit and wave', base);
  file.blanks = [
    { id: 'bench', label: 'The bench it sits on', kind: 'prop', prop: 'stone-bench', point: null },
  ];
  file.layers = [
    layer('l-1', 'reach', 'leftHand', { start: 0.1, end: 0.3, target: 'bench' }),
    layer('l-2', 'wave', 'rightLowerArm', { start: 0.5, end: 0.9, cycles: 2, amount: 25 }),
  ];
  return file;
}
