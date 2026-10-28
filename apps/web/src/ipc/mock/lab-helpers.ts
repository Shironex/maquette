import type { ConceptJob, Direction, NewObjectRefs, ProjectManifest, RefImage } from '../client';
import type { FakeLabOptions } from './lab';
import { clock } from '@/lib/clock';

export const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'project';

/** A new job waiting for a slot, with what it was given. */
export const queuedJob = (
  given: Pick<
    ConceptJob,
    'id' | 'runId' | 'project' | 'direction' | 'directionName' | 'index' | 'file' | 'title'
  > &
    Partial<Pick<ConceptJob, 'styleRevision'>>
): ConceptJob => ({
  styleRevision: null,
  ...given,
  state: 'queued',
  queuedAtMs: clock.nowMs(),
  startedAtMs: null,
  finishedAtMs: null,
  message: null,
  width: null,
  height: null,
  logTail: [],
  retryAtMs: null,
  usageLimit: false,
  retries: 0,
});

export const finished = (job: ConceptJob) =>
  job.state !== 'queued' && job.state !== 'running' && job.state !== 'rate-limited';

/** A job Retry runs again: failed, cancelled or waiting out a rate limit. */
export const retryable = (job: ConceptJob) =>
  job.state === 'failed' || job.state === 'cancelled' || job.state === 'rate-limited';

/** A ref may stand in as a card's picture: none yet, or an older ref. */
export const refMayReplace = (thumbnail: string | null) =>
  thumbnail === null || thumbnail.startsWith('refs/');

/** Copies dropped images into the project's `refs/`, as the core does. */
export function importRefs(
  options: Pick<FakeLabOptions, 'projectDir' | 'files' | 'fail'>,
  refs: Map<string, RefImage[]>,
  project: string,
  paths: string[]
): string[] {
  const dir = options.projectDir(project);
  const out = paths.map(path => {
    const name = path.slice(path.lastIndexOf('/') + 1);
    if (!/\.(png|jpe?g|webp)$/i.test(name))
      options.fail('invalid', `${name} is not a PNG, JPEG or WebP image.`);
    options.files?.alias(`${dir}/refs/${name}`, path);
    return `refs/${name}`;
  });
  const known = refs.get(project) ?? [];
  refs.set(project, [
    ...known.filter(r => !out.includes(r.file)),
    ...out.map(file => ({ file, name: file.slice(5), bytes: 1.4e6 })),
  ]);
  return out;
}

/**
 * Where object refs land, as the core names it: "<source> objects", or
 * "... objects 2", "... objects 3" when another source or kind has the name.
 */
export function objectsDirectionName(
  directions: readonly Direction[],
  request: Pick<NewObjectRefs, 'sourceName' | 'source' | 'kind'>
): string {
  const base = request.sourceName.trim() || 'Concept';
  const numbered = (n: number) => {
    const suffix = n === 1 ? ' objects' : ` objects ${n}`;
    return `${base.slice(0, 60 - suffix.length).trimEnd()}${suffix}`;
  };
  const fits = (n: number) => {
    const found = directions.find(d => d.id === slug(numbered(n)));
    return !found || (found.source === request.source && found.kind === request.kind);
  };
  let n = 1;
  while (!fits(n)) n += 1;
  return numbered(n);
}

/** Cards an older Send picks to Refs left without a picture get the newest ref. */
export function healThumbnails(
  p: ProjectManifest,
  sentFor: (project: string, asset: string) => string[],
  save: (next: ProjectManifest) => ProjectManifest
): ProjectManifest {
  let changed = false;
  const assets = (p.assets ?? []).map(a => {
    if (a.thumbnail !== null || a.versions.length > 0) return a;
    const newest = sentFor(p.id, a.name).at(-1);
    if (!newest || !refMayReplace(a.thumbnail)) return a;
    changed = true;
    return { ...a, thumbnail: newest };
  });
  return changed ? save({ ...p, assets }) : p;
}
