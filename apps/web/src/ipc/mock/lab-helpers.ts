import type { ConceptJob } from '../client';

export const slug = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'project';

export const finished = (job: ConceptJob) => job.state !== 'queued' && job.state !== 'running';

/** A ref may stand in as a card's picture: none yet, or an older ref. */
export const refMayReplace = (thumbnail: string | null) =>
  thumbnail === null || thumbnail.startsWith('refs/');
