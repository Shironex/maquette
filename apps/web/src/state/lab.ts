/** Pure helpers for the Concept lab: grid tiles, job lines and summaries. */
import type { Status } from '@maquette/ui';
import type {
  Aspect,
  ConceptEvent,
  ConceptImage,
  ConceptJob,
  ConceptJobState,
  Direction,
  LabView,
} from '@/ipc/client';
import { formatClock } from '@/lib/format';

export const ASPECTS: readonly { value: Aspect; label: string }[] = [
  { value: '16:9', label: '16:9' },
  { value: '3:2', label: '3:2' },
  { value: '1:1', label: '1:1' },
  { value: '3:4', label: '3:4' },
  { value: '9:16', label: '9:16' },
];

/** CSS `aspect-ratio` for a direction's tiles. */
export function aspectCss(aspect: Aspect): string {
  return aspect.replace(':', ' / ');
}

export const MAX_IMAGES = 12;
export const MAX_REFS = 8;

/** "Concept 05" */
export function conceptLabel(index: number): string {
  return `Concept ${String(index).padStart(2, '0')}`;
}

/** An object ref's own name, or "Concept 05". */
export function tileLabel(item: Pick<ConceptImage, 'index' | 'title'>): string {
  return item.title ?? conceptLabel(item.index);
}

/** The most objects one "Make object refs" takes; the core says the same. */
export const MAX_OBJECTS = 12;
/** The longest object name, which is also the asset name; the core says the same. */
export const MAX_OBJECT_CHARS = 60;
/** The longest note on an object; the core says the same. */
export const MAX_NOTE_CHARS = 400;

/** True for a direction of object refs made from a concept. */
export function isObjects(direction: Pick<Direction, 'source'>): boolean {
  return direction.source !== null;
}

/** "Veil gate props objects", the direction the object refs land in. */
export function objectsDirectionName(sourceName: string): string {
  const base = sourceName.trim() || 'Concept';
  return `${base.slice(0, 60 - ' objects'.length).trimEnd()} objects`;
}

/**
 * Adds typed or pasted objects: one per line, comma or semicolon, trimmed,
 * starting with a capital, without repeats in any case, at most 12. A name
 * over the limit is left out, never cut: see `longestObject`.
 */
export function addObjects(current: readonly string[], text: string): string[] {
  const out = [...current];
  for (const part of objectParts(text)) {
    // "lantern post" names the asset "Lantern post".
    const name = part.charAt(0).toUpperCase() + part.slice(1);
    if (name.length > MAX_OBJECT_CHARS) continue;
    if (out.some(o => o.toLowerCase() === name.toLowerCase())) continue;
    if (out.length >= MAX_OBJECTS) break;
    out.push(name);
  }
  return out;
}

function objectParts(text: string): string[] {
  return text
    .split(/[\n,;]/)
    .map(part => part.split(/\s+/).filter(Boolean).join(' '))
    .filter(Boolean);
}

/** The length of the longest name in typed text, to show against the limit. */
export function longestObject(text: string): number {
  return Math.max(0, ...objectParts(text).map(part => part.length));
}

/**
 * Renames an object and sets its notes. The notes follow the name; an empty
 * note is dropped. Null when the new name is empty, too long or taken.
 */
export function editObject(
  objects: readonly string[],
  notes: Readonly<Record<string, string>>,
  name: string,
  next: { name: string; notes: string }
): { objects: string[]; notes: Record<string, string> } | null {
  const words = next.name.split(/\s+/).filter(Boolean).join(' ');
  const renamed = words.charAt(0).toUpperCase() + words.slice(1);
  const text = next.notes.trim();
  if (!renamed || renamed.length > MAX_OBJECT_CHARS || text.length > MAX_NOTE_CHARS) return null;
  if (objects.some(o => o !== name && o.toLowerCase() === renamed.toLowerCase())) return null;
  const rest = Object.fromEntries(Object.entries(notes).filter(([key]) => key !== name));
  return {
    objects: objects.map(o => (o === name ? renamed : o)),
    notes: text ? { ...rest, [renamed]: text } : rest,
  };
}

/** "58s", "1m 04s", "12m 30s". */
export function formatElapsed(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 60) return `${s}s`;
  return `${Math.floor(s / 60)}m ${String(s % 60).padStart(2, '0')}s`;
}

const isLive = (state: ConceptJobState) =>
  state === 'queued' || state === 'running' || state === 'rate-limited';

/** True for a job Retry runs again: failed, cancelled, or waiting out a rate limit. */
export const canRetry = (job: Pick<ConceptJob, 'state'>): boolean =>
  job.state === 'failed' || job.state === 'cancelled' || job.state === 'rate-limited';

/**
 * "Waiting, rate limited, retries at 09:05", or for Codex's usage limit
 * "Waiting for the Codex usage limit until about 16:22".
 */
export function rateLimitedLine(job: Pick<ConceptJob, 'retryAtMs' | 'usageLimit'>): string {
  if (job.usageLimit)
    return job.retryAtMs === null
      ? 'Waiting for the Codex usage limit'
      : `Waiting for the Codex usage limit until about ${formatClock(job.retryAtMs)}`;
  return job.retryAtMs === null
    ? 'Waiting, rate limited'
    : `Waiting, rate limited, retries at ${formatClock(job.retryAtMs)}`;
}

export function jobStatus(job: ConceptJob): Status {
  switch (job.state) {
    case 'done':
      return 'done';
    case 'running':
      return 'running';
    case 'failed':
    case 'cancelled':
      return 'failed';
    default:
      return 'queued';
  }
}

/** The second line of a queue row. */
export function jobMeta(job: ConceptJob, now: number): string {
  const took = (end: number | null) =>
    job.startedAtMs !== null ? formatElapsed((end ?? now) - job.startedAtMs) : null;
  switch (job.state) {
    case 'queued':
      return 'Waiting for a free slot';
    case 'running':
      return `Generating · ${took(null) ?? '0s'}`;
    case 'done': {
      const size = job.width && job.height ? `${job.width} × ${job.height}` : 'Saved';
      const time = took(job.finishedAtMs);
      return time ? `${size} · ${time}` : size;
    }
    case 'cancelled':
      return 'Cancelled';
    case 'rate-limited':
      return rateLimitedLine(job);
    default:
      return job.message ?? 'Failed';
  }
}

/** One cell of the results grid: a finished image or a job still going. */
export type Tile =
  | { kind: 'image'; key: string; image: ConceptImage }
  | { kind: 'job'; key: string; job: ConceptJob };

/** The direction's images in order, then its queued and running jobs. */
export function tiles(direction: Direction, jobs: readonly ConceptJob[]): Tile[] {
  const images: Tile[] = [...direction.images]
    .sort((a, b) => a.index - b.index)
    .map(image => ({ kind: 'image', key: image.file, image }));
  const live: Tile[] = jobs
    .filter(j => j.direction === direction.id && isLive(j.state))
    .filter(j => !direction.images.some(i => i.file === j.file))
    .sort((a, b) => a.index - b.index)
    .map(job => ({ kind: 'job', key: job.id, job }));
  return [...images, ...live];
}

/** "6 concepts · 2 starred · 1 picked", counting the images still generating. */
export function directionSummary(direction: Direction, jobs: readonly ConceptJob[]): string {
  const going = jobs.filter(
    j =>
      j.direction === direction.id &&
      isLive(j.state) &&
      !direction.images.some(i => i.file === j.file)
  ).length;
  const n = direction.images.length + going;
  const noun = isObjects(direction) ? ['object', 'objects'] : ['concept', 'concepts'];
  const parts = [`${n} ${n === 1 ? noun[0] : noun[1]}`];
  const starred = direction.images.filter(i => i.starred).length;
  const picked = direction.images.filter(i => i.picked).length;
  if (starred) parts.push(`${starred} starred`);
  if (picked) parts.push(`${picked} picked`);
  return parts.join(' · ');
}

export interface QueueSummary {
  done: number;
  total: number;
  running: number;
  queued: number;
  failed: number;
}

export function queueSummary(jobs: readonly ConceptJob[]): QueueSummary {
  return {
    done: jobs.filter(j => j.state === 'done').length,
    total: jobs.length,
    running: jobs.filter(j => j.state === 'running').length,
    queued: jobs.filter(j => j.state === 'queued' || j.state === 'rate-limited').length,
    failed: jobs.filter(j => j.state === 'failed').length,
  };
}

/** "2 running, 4 at a time", "1 waiting, 4 at a time", "Idle, 4 at a time". */
export function queueFooter(summary: QueueSummary, concurrency: number): string {
  const slots = `${concurrency} at a time`;
  if (summary.running && summary.queued)
    return `${summary.running} running, ${summary.queued} waiting, ${slots}`;
  if (summary.running) return `${summary.running} running, ${slots}`;
  if (summary.queued) return `${summary.queued} waiting, ${slots}`;
  return `Idle, ${slots}`;
}

/** Folds one service event into the view. */
export function applyConceptEvent(view: LabView, event: ConceptEvent): LabView {
  if (event.type === 'job') {
    const { job } = event;
    if (job.project !== view.project) return view;
    const known = view.jobs.some(j => j.id === job.id);
    return {
      ...view,
      jobs: known ? view.jobs.map(j => (j.id === job.id ? job : j)) : [...view.jobs, job],
    };
  }
  if (event.project !== view.project) return view;
  if (event.type === 'style') return { ...view, style: event.style };
  const others = view.directions.filter(d => d.id !== event.direction.id);
  return { ...view, directions: [event.direction, ...others] };
}

/** Replaces one direction in the view, keeping its place. */
export function withDirection(view: LabView, direction: Direction): LabView {
  const known = view.directions.some(d => d.id === direction.id);
  return {
    ...view,
    directions: known
      ? view.directions.map(d => (d.id === direction.id ? direction : d))
      : [direction, ...view.directions],
  };
}

/** The image an earlier direction shows in the list: its first pick, star or image. */
export function coverImage(direction: Direction): ConceptImage | null {
  return (
    direction.images.find(i => i.picked) ??
    direction.images.find(i => i.starred) ??
    direction.images[0] ??
    null
  );
}

/** "Frozen lake arena" and "frozen lake arena" are the same direction, as the Rust slug sees it. */
export function directionId(name: string): string {
  return (
    name
      .toLowerCase()
      .normalize('NFKD')
      .replace(/[̀-ͯ]/g, '')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'project'
  );
}

/** Joins a project relative file onto the project folder. */
export function inProject(projectDir: string, file: string): string {
  if (/^([a-z]:)?[\\/]/i.test(file)) return file;
  const sep = projectDir.includes('\\') && !projectDir.includes('/') ? '\\' : '/';
  return `${projectDir.replace(/[\\/]+$/, '')}${sep}${sep === '/' ? file : file.replace(/\//g, sep)}`;
}
