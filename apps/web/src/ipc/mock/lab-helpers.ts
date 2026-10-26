import type { ConceptJob, RefImage } from '../client';
import type { FakeLabOptions } from './lab';

export const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'project';

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
