/**
 * The name a By hand send asks for, as the core writes it
 * (`maquette-providers/src/naming.rs`):
 * `maq_<project>_<asset>_v<n>_<provider>`, with n the send's number in its
 * project. A part is the id itself when it is a plain slug within its cap,
 * else the start of its slug, `--` and six hex characters of its hash.
 */
import type { ProviderJob } from '../client';

const PROJECT_PART_MAX = 24;
const ASSET_PART_MAX = 40;
const TAG_MARK = '--';
const TAG_LEN = 6;

/** FNV-1a, 32 bit, over the id's UTF-8 bytes: the same hash the core takes. */
function fnv1a(text: string): number {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(text)) {
    hash = Math.imul(hash ^ byte, 0x01000193) >>> 0;
  }
  return hash;
}

const isPlain = (id: string): boolean =>
  /^[a-z0-9-]+$/.test(id) && !id.startsWith('-') && !id.endsWith('-') && !id.includes(TAG_MARK);

/** Lower case letters and digits, with a `-` for every run of anything else. */
const slugOf = (id: string): string =>
  id
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');

/** An id as one part of an export name, at most `max` characters. */
export function namePart(id: string, max: number, fallback: string): string {
  if (isPlain(id) && id.length <= max) return id;
  const keep = max - TAG_MARK.length - TAG_LEN;
  const head = (slugOf(id) || fallback).slice(0, keep).replace(/-+$/, '');
  const tag = (fnv1a(id) & 0xffffff).toString(16).padStart(TAG_LEN, '0');
  return `${head}${TAG_MARK}${tag}`;
}

/** The export name of `job`, n from its id `<asset>-<n>`. */
export function exportName(job: Pick<ProviderJob, 'id' | 'project' | 'asset' | 'service'>): string {
  const n = job.id.startsWith(`${job.asset}-`) ? job.id.slice(job.asset.length + 1) : job.id;
  const project = namePart(job.project, PROJECT_PART_MAX, 'project');
  const asset = namePart(job.asset, ASSET_PART_MAX, 'asset');
  return `maq_${project}_${asset}_v${n}_${job.service}`;
}

/**
 * The next send id of `asset` in `project`, as the core numbers them: one
 * past the highest number the asset has in that project, whatever other
 * projects sent.
 */
export function nextJobId(
  jobs: readonly Pick<ProviderJob, 'id' | 'project'>[],
  project: string,
  asset: string
): string {
  const numbers = jobs
    .filter(j => j.project === project && j.id.startsWith(`${asset}-`))
    .map(j => j.id.slice(asset.length + 1))
    .filter(n => /^\d+$/.test(n))
    .map(Number);
  return `${asset}-${Math.max(0, ...numbers) + 1}`;
}
