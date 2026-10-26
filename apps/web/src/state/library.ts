/**
 * The Library view's rules: filters, sorting, counts, card lines and the
 * update a Board asset has waiting. The library copies models into
 * projects; an asset's `origin` names the entry and version it came from.
 */
import type {
  Asset,
  GameFormat,
  LibraryItem,
  LibrarySource,
  LibraryVersion,
  LibraryView,
  ProjectManifest,
} from '@/ipc/client';
import { formatAgo, formatBytes, formatK } from '@/lib/format';

export const SOURCE_LABEL: Record<LibrarySource, string> = {
  tripo: 'Tripo',
  meshy: 'Meshy',
  'by-hand': 'By hand',
  built: 'Built',
  imported: 'Imported',
};

export const SOURCES: readonly LibrarySource[] = ['tripo', 'meshy', 'built', 'by-hand', 'imported'];
export const FORMATS: readonly GameFormat[] = ['glb', 'obj'];

export type LibrarySort = 'recent' | 'used' | 'name';
export type LibraryLayout = 'grid' | 'list';

/** Picks "Not used" in the Used in filter. */
export const NOT_USED = '';

export interface LibraryFilters {
  query: string;
  tag: string | null;
  source: LibrarySource | null;
  format: GameFormat | null;
  /** A project id, `NOT_USED`, or null for every model. */
  usedIn: string | null;
}

export const NO_FILTERS: LibraryFilters = {
  query: '',
  tag: null,
  source: null,
  format: null,
  usedIn: null,
};

export const latestOf = (item: LibraryItem): LibraryVersion | undefined =>
  item.entry.versions.at(-1);

/** The projects that use an entry, each once. */
export function projectsOf(item: LibraryItem): string[] {
  return [...new Set(item.uses.map(u => u.project))];
}

function matchesQuery(item: LibraryItem, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const text = [item.entry.name, ...item.entry.tags, ...item.uses.map(u => u.projectName)]
    .join(' ')
    .toLowerCase();
  return words.every(w => text.includes(w));
}

export function filterItems(items: readonly LibraryItem[], f: LibraryFilters): LibraryItem[] {
  return items.filter(item => {
    if (f.query && !matchesQuery(item, f.query)) return false;
    if (f.tag && !item.entry.tags.includes(f.tag)) return false;
    if (f.source && latestOf(item)?.source !== f.source) return false;
    if (f.format && item.format !== f.format) return false;
    if (f.usedIn === NOT_USED) return item.uses.length === 0;
    if (f.usedIn && !item.uses.some(u => u.project === f.usedIn)) return false;
    return true;
  });
}

export function sortItems(items: readonly LibraryItem[], sort: LibrarySort): LibraryItem[] {
  const byName = (a: LibraryItem, b: LibraryItem) => a.entry.name.localeCompare(b.entry.name, 'en');
  return [...items].sort((a, b) => {
    if (sort === 'recent')
      return b.entry.updatedAt.localeCompare(a.entry.updatedAt) || byName(a, b);
    if (sort === 'used') return projectsOf(b).length - projectsOf(a).length || byName(a, b);
    return byName(a, b);
  });
}

/** Each value to how many entries have it, most first then by name. */
function counted(values: string[]): [string, number][] {
  const counts = new Map<string, number>();
  for (const v of values) counts.set(v, (counts.get(v) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0], 'en'));
}

export const tagCounts = (items: readonly LibraryItem[]) =>
  counted(items.flatMap(i => i.entry.tags));

export function sourceCounts(items: readonly LibraryItem[]): [LibrarySource, number][] {
  return SOURCES.map(s => [s, items.filter(i => latestOf(i)?.source === s).length]);
}

export function formatCounts(items: readonly LibraryItem[]): [GameFormat, number][] {
  return FORMATS.map(f => [f, items.filter(i => i.format === f).length]);
}

/** Every project that uses a model, by name, with how many models it uses. */
export function projectCounts(items: readonly LibraryItem[]): [string, string, number][] {
  const names = new Map<string, string>();
  for (const u of items.flatMap(i => i.uses)) names.set(u.project, u.projectName);
  return [...names]
    .map(([id, name]): [string, string, number] => [
      id,
      name,
      items.filter(i => i.uses.some(u => u.project === id)).length,
    ])
    .sort((a, b) => b[2] - a[2] || a[1].localeCompare(b[1], 'en'));
}

export interface LibraryStats {
  models: number;
  /** Used in two projects or more. */
  shared: number;
  saved: number;
  updates: number;
}

export function libraryStats(items: readonly LibraryItem[]): LibraryStats {
  return {
    models: items.length,
    shared: items.filter(i => projectsOf(i).length > 1).length,
    saved: items.reduce((sum, i) => sum + (i.saved ?? 0), 0),
    updates: items.reduce((sum, i) => sum + i.uses.filter(u => u.update).length, 0),
  };
}

/** "v3 · Built", "v2 · Tripo, rigged". */
export function itemLine(item: LibraryItem): string {
  const latest = latestOf(item);
  if (!latest) return 'No model';
  const rigged = (latest.stats?.joints ?? 0) > 0 ? ', rigged' : '';
  return `v${latest.version} · ${SOURCE_LABEL[latest.source]}${rigged}`;
}

/** "2 projects", "Not used yet". */
export function usedLine(item: LibraryItem): string {
  const n = projectsOf(item).length;
  if (n === 0) return 'Not used yet';
  return n === 1 ? '1 project' : `${n} projects`;
}

/** What it cost, or what reuse saved. */
export function creditLine(item: LibraryItem): { text: string; saved: boolean } {
  const saved = item.saved ?? 0;
  const spent = item.spent ?? 0;
  if (saved > 0) return { text: `${formatK(saved)} saved by reuse`, saved: true };
  const source = latestOf(item)?.source;
  if (spent > 0) return { text: `${formatK(spent)} credits`, saved: false };
  if (source === 'built') return { text: 'no credits, built', saved: false };
  if (source === 'by-hand') return { text: 'no credits, by hand', saved: false };
  return { text: 'no credits', saved: false };
}

/** "Tripo · 30 credits · 2 days ago". */
export function versionLine(v: LibraryVersion, nowMs: number): string {
  const credits = v.credits ? ` · ${formatK(v.credits)} credits` : '';
  return `${SOURCE_LABEL[v.source]}${credits} · ${formatAgo(Date.parse(v.createdAt), nowMs)}`;
}

/** "48k triangles · 1.2 MB". */
export function numbersLine(item: LibraryItem): string | null {
  const s = latestOf(item)?.stats;
  if (!s) return null;
  return `${formatK(s.triangles)} triangles · ${formatBytes(s.bytes ?? 0)}`;
}

/** The newest picture an entry has, as a path for `fileUrl`. */
export function pictureOf(item: LibraryItem): string | null {
  const shown = [...item.entry.versions].reverse().find(v => v.thumbnail)?.thumbnail;
  const picture = shown ?? item.entry.refs[0];
  return picture ? `${item.dir}/${picture}` : null;
}

/** The update an asset that came from the library has waiting, if any. */
export interface WaitingUpdate {
  entry: LibraryItem;
  from: number;
  to: number;
}

export function updateFor(view: LibraryView | null, asset: Asset): WaitingUpdate | null {
  const origin = asset.origin;
  if (!view || !origin) return null;
  const entry = view.items.find(i => i.entry.id === origin.entry);
  const to = entry ? latestOf(entry)?.version : undefined;
  if (!entry || to === undefined || to <= origin.version) return null;
  return { entry, from: origin.version, to };
}

/**
 * Why `item` cannot go into `project`, if it cannot: a website builds its
 * scene from .glb models. A game takes either; its next run exports it.
 */
export function useBlocked(item: LibraryItem, project: ProjectManifest | null): string | null {
  if (!project) return 'Open a project first';
  if ((project.target ?? 'website') === 'website' && item.format === 'obj') {
    return `${project.name} builds its scene from .glb models, and this one is .obj`;
  }
  return null;
}

/** What the open project does with a model when it comes in, said plainly. */
export function useNote(item: LibraryItem, project: ProjectManifest | null): string {
  if (project?.target === 'game') {
    const wanted = project.game?.format ?? 'glb';
    return item.format === wanted
      ? `The next run exports it into the game as .${wanted}.`
      : `This game takes .${wanted}. The model comes in as .${item.format ?? 'glb'} and the next run's Game export writes it as .${wanted}; nothing is converted now.`;
  }
  return 'The model, its picture, refs, real size and fix-ups copy into the project. Reuse spends nothing; only a new send to 3D does.';
}
