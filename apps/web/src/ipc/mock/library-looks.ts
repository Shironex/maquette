/**
 * Look-alikes and the add a whole project plan in mock mode. Mock mode has
 * no pictures to compare, so how alike two models look comes from the
 * fixtures' tables; names pair the way the core pairs them.
 */
import type {
  AddManyPlan,
  LibraryEntry,
  LookAlikePair,
  LookAlikes,
  PlannedAdd,
  ProjectManifest,
} from '../client';
import { clock } from '@/lib/clock';
import type { LibraryState } from './create-library-batch-handlers';
import { CUT_SHORT, HOLLOW_LOOKS } from './hollow-vale-fixtures';
import { NO_PICTURE, SCALE_LOOKS } from './library-scale-fixtures';
import { slugify } from './backend-options';

/** The weights the Inbox gives outline, colour and box ratio. */
const WEIGHTS = { shape: 0.45, colour: 0.35, ratio: 0.2 };

export const pairId = (a: string, b: string) => [a, b].sort().join('|');

/** A name as pairs compare it: lowercase, trimmed, a trailing number dropped. */
export const baseName = (name: string) =>
  name
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+\d+$/, '');

const older = (x: LibraryEntry, y: LibraryEntry) => (x.createdAt <= y.createdAt ? [x, y] : [y, x]);

export function lookAlikes(state: LibraryState): LookAlikes {
  const started = performance.now();
  const byId = new Map(state.entries.map(e => [e.id, e]));
  const pairs: LookAlikePair[] = [];
  const taken = new Set<string>();
  for (const look of SCALE_LOOKS) {
    const a = byId.get(look.a);
    const b = byId.get(look.b);
    if (!a || !b || state.decided.has(pairId(a.id, b.id))) continue;
    const score =
      WEIGHTS.shape * look.shape + WEIGHTS.colour * look.colour + WEIGHTS.ratio * look.ratio;
    const sameName = baseName(a.name) === baseName(b.name);
    if (score < (state.settings.threshold ?? 0.9) && !sameName) continue;
    const [first, second] = older(a, b);
    taken.add(pairId(a.id, b.id));
    pairs.push({
      kind: score >= (state.settings.threshold ?? 0.9) ? 'same-look' : 'same-name',
      a: first!.id,
      b: second!.id,
      score,
      shape: look.shape,
      colour: look.colour,
      ratio: look.ratio,
    });
  }
  const byName = new Map<string, LibraryEntry[]>();
  for (const e of state.entries)
    byName.set(baseName(e.name), [...(byName.get(baseName(e.name)) ?? []), e]);
  for (const group of byName.values()) {
    for (let i = 0; i < group.length; i++) {
      for (let j = i + 1; j < group.length; j++) {
        const id = pairId(group[i]!.id, group[j]!.id);
        if (taken.has(id) || state.decided.has(id)) continue;
        const [first, second] = older(group[i]!, group[j]!);
        pairs.push({
          kind: 'same-name',
          a: first!.id,
          b: second!.id,
          score: null,
          shape: null,
          colour: null,
          ratio: null,
        });
      }
    }
  }
  pairs.sort((x, y) =>
    x.kind === y.kind ? (y.score ?? 0) - (x.score ?? 0) : x.kind === 'same-look' ? -1 : 1
  );
  const merges = [...state.removed.values()]
    .flatMap(r => (r.merge ? [r.merge] : []))
    .sort((x, y) => y.at.localeCompare(x.at))
    .slice(0, 5);
  return {
    pairs,
    unseen: state.entries
      .filter(e => NO_PICTURE.has(e.id) || !e.versions.at(-1)?.thumbnail)
      .map(e => e.id),
    merges,
    settings: state.settings,
    scannedAt: clock.now().toISOString(),
    tookMs: Math.max(1, Math.round(performance.now() - started)),
  };
}

export function addManyPlan(state: LibraryState, project: ProjectManifest): AddManyPlan {
  const hashes = new Map(state.entries.flatMap(e => e.versions.map(v => [v.hash, e] as const)));
  const names = new Map<string, LibraryEntry>();
  for (const e of state.entries) if (!names.has(baseName(e.name))) names.set(baseName(e.name), e);
  const assets = (project.assets ?? []).flatMap((a): PlannedAdd[] => {
    const latest = a.versions.at(-1);
    if (!latest) return [];
    const base = {
      asset: a.id,
      name: a.name,
      kind: a.kind,
      file: latest.file,
      triangles: latest.stats?.triangles ?? null,
      entry: null,
      entryName: null,
      looks: null,
      reason: null,
    };
    if (CUT_SHORT.has(a.id)) {
      return [
        {
          ...base,
          state: 'unreadable',
          reason: 'is cut short: the file ends before its header says.',
        },
      ];
    }
    const same = hashes.get(`${project.id}/${latest.file}`);
    if (same) return [{ ...base, state: 'same-file', entry: same.id, entryName: same.name }];
    const own = state.entries.find(
      e =>
        e.id === a.origin?.entry ||
        e.versions.some(v => v.from.project === project.id && v.from.asset === a.id)
    );
    if (own) return [{ ...base, state: 'next-version', entry: own.id, entryName: own.name }];
    const clash = names.get(baseName(a.name));
    if (clash) {
      return [
        {
          ...base,
          state: 'name-clash',
          entry: clash.id,
          entryName: clash.name,
          looks: HOLLOW_LOOKS[a.id] ?? null,
        },
      ];
    }
    return [{ ...base, state: 'new' }];
  });
  return {
    project: project.id,
    projectName: project.name,
    tag: slugify(project.name),
    checkOnAdd: state.settings.checkOnAdd,
    assets,
  };
}
