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

/** Latin letters with marks and what the core folds them to (`project/slug.rs`). */
const FOLDS: [string, string][] = [
  ['àáâãäåąāă', 'a'],
  ['æ', 'ae'],
  ['çćčĉ', 'c'],
  ['ďđð', 'd'],
  ['èéêëęēě', 'e'],
  ['ğĝ', 'g'],
  ['ìíîïīı', 'i'],
  ['łľĺ', 'l'],
  ['ñńň', 'n'],
  ['òóôõöøōő', 'o'],
  ['œ', 'oe'],
  ['řŕ', 'r'],
  ['śšşŝ', 's'],
  ['ß', 'ss'],
  ['ťţ', 't'],
  ['þ', 'th'],
  ['ùúûüūůű', 'u'],
  ['ýÿ', 'y'],
  ['źżž', 'z'],
];
const FOLD = new Map(FOLDS.flatMap(([from, to]) => [...from].map(c => [c, to] as const)));

const fold = (c: string): string =>
  /^[A-Za-z0-9]$/.test(c) ? c.toLowerCase() : (FOLD.get([...c.toLowerCase()][0] ?? c) ?? '');

/**
 * The core's latin slug: letters with marks folded, a `-` where space or
 * ASCII punctuation sat between kept characters, anything else dropped.
 */
function slugOf(id: string): string {
  let slug = '';
  let dash = false;
  for (const c of id) {
    const folded = fold(c);
    if (!folded) {
      if (/^\s$/u.test(c) || /^[!-/:-@[-`{-~]$/.test(c)) dash = true;
      continue;
    }
    if (dash && slug) slug += '-';
    dash = false;
    slug += folded;
  }
  return slug.replace(/-+$/, '');
}

/** An id as one part of an export name, at most `max` characters. */
export function namePart(id: string, max: number, fallback: string): string {
  if (isPlain(id) && id.length <= max) return id;
  const keep = max - TAG_MARK.length - TAG_LEN;
  const head = (slugOf(id) || fallback).slice(0, keep).replace(/-+$/, '');
  const tag = (fnv1a(id) & 0xffffff).toString(16).padStart(TAG_LEN, '0');
  return `${head}${TAG_MARK}${tag}`;
}

/**
 * The `<n>` of a name, as the core reads it: the send's number as a number,
 * else what follows the asset in the id.
 */
function numberText(id: string, asset: string): string {
  const digits = id.startsWith(`${asset}-`) ? id.slice(asset.length + 1) : '';
  if (/^\d+$/.test(digits) && Number(digits) <= 0xffffffff) return String(Number(digits));
  return id.startsWith(asset) ? id.slice(asset.length) : id;
}

/** The export name of `job`, n from its id `<asset>-<n>`. */
export function exportName(job: Pick<ProviderJob, 'id' | 'project' | 'asset' | 'service'>): string {
  const n = numberText(job.id, job.asset);
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
