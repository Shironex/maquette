/**
 * The Library search. Plain words match a model's name, tags, the projects
 * that use it and what made it, all of them. A few words narrow further:
 *
 * - `tag:lights` only models with that tag
 * - `in:frostveil` only models a project with that name or id uses
 * - `by:tripo` only models whose newest version that source made
 * - `<5k`, `>20k`, `tris<5000` the newest version's triangles
 */
import type { LibraryItem } from '@/ipc/client';
import { SOURCE_LABEL } from './library';

export interface LibraryQuery {
  words: string[];
  tags: string[];
  projects: string[];
  sources: string[];
  /** Triangles above this, or null. */
  over: number | null;
  /** Triangles below this, or null. */
  under: number | null;
}

const TRIANGLES = /^(?:tris|triangles)?([<>])=?(\d+(?:\.\d+)?)([km])?$/;
const SCALE = { k: 1_000, m: 1_000_000 } as const;

export function parseQuery(text: string): LibraryQuery {
  const q: LibraryQuery = {
    words: [],
    tags: [],
    projects: [],
    sources: [],
    over: null,
    under: null,
  };
  for (const raw of text.toLowerCase().split(/\s+/).filter(Boolean)) {
    const [head, ...rest] = raw.split(':');
    const value = rest.join(':');
    const tris = TRIANGLES.exec(raw);
    if (tris) {
      const n = Number(tris[2]) * (tris[3] ? SCALE[tris[3] as 'k' | 'm'] : 1);
      if (tris[1] === '<') q.under = n;
      else q.over = n;
    } else if (value && head === 'tag') q.tags.push(value);
    else if (value && (head === 'in' || head === 'project')) q.projects.push(value);
    else if (value && (head === 'by' || head === 'source')) q.sources.push(value);
    else q.words.push(raw);
  }
  return q;
}

/** True when the search says nothing. */
export const isEmptyQuery = (q: LibraryQuery) =>
  q.words.length + q.tags.length + q.projects.length + q.sources.length === 0 &&
  q.over === null &&
  q.under === null;

function sourceText(item: LibraryItem): string {
  const source = item.entry.versions.at(-1)?.source;
  return source ? `${source} ${SOURCE_LABEL[source]}`.toLowerCase() : '';
}

export function matchesQuery(item: LibraryItem, q: LibraryQuery): boolean {
  const { entry, uses } = item;
  const projects = uses.flatMap(u => [u.project, u.projectName.toLowerCase()]);
  const source = sourceText(item);
  const text = [entry.name.toLowerCase(), ...entry.tags, ...projects, source].join(' ');
  if (!q.words.every(w => text.includes(w))) return false;
  if (!q.tags.every(t => entry.tags.some(tag => tag.includes(t)))) return false;
  if (!q.projects.every(p => projects.some(name => name.includes(p)))) return false;
  if (!q.sources.every(s => source.includes(s))) return false;
  const tris = entry.versions.at(-1)?.stats?.triangles;
  if (q.over !== null && (tris === undefined || tris <= q.over)) return false;
  if (q.under !== null && (tris === undefined || tris >= q.under)) return false;
  return true;
}
