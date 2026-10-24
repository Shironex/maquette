/**
 * Sending several assets at once: the Board selection, the batch draft
 * and request, and the By hand checklist and Downloads matches.
 */
import type {
  BatchOptions,
  BatchRequest,
  DownloadsScan,
  KeyStatus,
  MeshyPreset,
  PresetVariant,
  Provider,
  ProviderId,
  ModelOption,
  ProviderJob,
  TripoPreset,
} from '@/ipc/client';
import {
  meshyChanges,
  meshyProblem,
  meshySummary,
  tripoChanges,
  tripoProblem,
  tripoSummary,
} from './provider-presets';
import { PROVIDER_LABEL, SERVICE_LABEL } from './providers';

/** The most assets one batch takes; the core says the same. */
export const MAX_BATCH = 24;

/**
 * The selection after a click on a card's select box. With `range` (Shift
 * held) every card between the last clicked one and this one follows it.
 */
export function nextSelection(
  selected: readonly string[],
  order: readonly string[],
  id: string,
  on: boolean,
  range: boolean,
  anchor: string | null
): string[] {
  const from = anchor ? order.indexOf(anchor) : -1;
  const to = order.indexOf(id);
  const span =
    range && from >= 0 && to >= 0 ? order.slice(Math.min(from, to), Math.max(from, to) + 1) : [id];
  const next = new Set(selected);
  for (const item of span) {
    if (on) next.add(item);
    else next.delete(item);
  }
  // Keep the Board's order, so the batch lists the cards as they stand.
  return order.filter(item => next.has(item));
}

export interface BatchItemDraft {
  asset: string;
  name: string;
  variant: PresetVariant;
  images: string[];
  thumbnail: string | null;
  /**
   * Only the fields this asset changed, over the batch preset of its kind.
   * Empty means it follows the batch, so a later edit of the batch settings
   * still reaches every field it did not change.
   */
  tripo: Partial<TripoPreset>;
  meshy: Partial<MeshyPreset>;
}

export interface BatchDraft {
  provider: Provider;
  service: ProviderId;
  items: BatchItemDraft[];
  tripo: Record<PresetVariant, TripoPreset>;
  meshy: Record<PresetVariant, MeshyPreset>;
}

/** A draft from the batch options and the saved presets. */
export function batchDraft(
  options: BatchOptions,
  tripo: Record<PresetVariant, TripoPreset>,
  meshy: Record<PresetVariant, MeshyPreset>,
  provider: Provider = 'by-hand'
): BatchDraft {
  return {
    provider,
    service: provider === 'meshy' ? 'meshy' : 'tripo',
    items: options.items.map(item => ({
      asset: item.asset.id,
      name: item.asset.name,
      variant: item.variant,
      images: item.suggested.slice(0, 4),
      thumbnail: item.asset.thumbnail,
      tripo: {},
      meshy: {},
    })),
    tripo: structuredClone(tripo),
    meshy: structuredClone(meshy),
  };
}

/** The presets the items use, props first. */
export function variantsUsed(draft: Pick<BatchDraft, 'items'>): PresetVariant[] {
  const used = new Set(draft.items.map(i => i.variant));
  return (['prop', 'character'] as const).filter(v => used.has(v));
}

/** The fields of `next` that differ from `base`. */
export function presetChanges<T extends object>(base: T, next: T): Partial<T> {
  const out: Partial<T> = {};
  for (const key of Object.keys(base) as (keyof T)[]) {
    if (JSON.stringify(base[key]) !== JSON.stringify(next[key])) out[key] = next[key];
  }
  return out;
}

/** The Tripo preset one asset sends with: its changes over the batch preset. */
export function itemTripo(draft: BatchDraft, item: BatchItemDraft): TripoPreset {
  return { ...draft.tripo[item.variant], ...item.tripo };
}

export function itemMeshy(draft: BatchDraft, item: BatchItemDraft): MeshyPreset {
  return { ...draft.meshy[item.variant], ...item.meshy };
}

/** The changed fields of an asset for the draft's service. */
function itemChanges(draft: BatchDraft, item: BatchItemDraft): object {
  return draft.service === 'tripo' ? item.tripo : item.meshy;
}

/** True when the asset changes at least one setting of its own. */
export function isCustom(draft: BatchDraft, item: BatchItemDraft): boolean {
  return Object.keys(itemChanges(draft, item)).length > 0;
}

/** "12,000 faces, no PBR": what an asset changes from the batch. */
export function itemDiff(
  draft: BatchDraft,
  item: BatchItemDraft,
  catalog: Pick<Catalog, 'tripoModels' | 'meshyModels'>
): string {
  const changes =
    draft.service === 'tripo'
      ? tripoChanges(item.tripo, itemTripo(draft, item), catalog.tripoModels)
      : meshyChanges(item.meshy, itemMeshy(draft, item), catalog.meshyModels);
  return changes.join(', ');
}

type Catalog = { tripoModels: readonly ModelOption[]; meshyModels: readonly ModelOption[] };

/** The first preset problem among the batch's presets and the assets' own. */
export function batchProblem(draft: BatchDraft, catalog: Catalog): string | null {
  const tripo = draft.service === 'tripo';
  for (const item of draft.items) {
    const custom = isCustom(draft, item);
    const problem = tripo
      ? tripoProblem(itemTripo(draft, item), catalog.tripoModels)
      : meshyProblem(itemMeshy(draft, item), catalog.meshyModels);
    if (problem) return custom ? `${item.name}: ${problem}` : problem;
  }
  return null;
}

/**
 * The settings lines of a batch: one per kind with assets that follow the
 * batch, then one per customised asset with just what it changes.
 */
export function batchLines(draft: BatchDraft, catalog: Catalog): string[] {
  const label = { prop: 'Props', character: 'Characters' } as const;
  const kinds = variantsUsed({
    items: draft.items.filter(i => !isCustom(draft, i)),
  }).map(
    v =>
      `${label[v]}: ${
        draft.service === 'tripo'
          ? tripoSummary(draft.tripo[v], catalog.tripoModels)
          : meshySummary(draft.meshy[v], catalog.meshyModels)
      }`
  );
  const own = draft.items
    .filter(i => isCustom(draft, i))
    .map(i => `${i.name}: ${itemDiff(draft, i, catalog)}`);
  return [...kinds, ...own];
}

export function batchRequest(project: string, draft: BatchDraft): BatchRequest {
  const used = variantsUsed(draft);
  const tripo = draft.service === 'tripo';
  const pick = <T>(on: boolean, variant: PresetVariant, preset: T): T | null =>
    on && used.includes(variant) ? preset : null;
  return {
    project,
    provider: draft.provider,
    service: draft.service,
    items: draft.items.map(i => {
      const own = isCustom(draft, i);
      return {
        asset: i.asset,
        variant: i.variant,
        images: i.images,
        tripo: own && tripo ? itemTripo(draft, i) : null,
        meshy: own && !tripo ? itemMeshy(draft, i) : null,
      };
    }),
    tripoProp: pick(tripo, 'prop', draft.tripo.prop),
    tripoCharacter: pick(tripo, 'character', draft.tripo.character),
    meshyProp: pick(!tripo, 'prop', draft.meshy.prop),
    meshyCharacter: pick(!tripo, 'character', draft.meshy.character),
  };
}

/** Why the batch cannot go yet, or null. */
export function batchBlocker(
  draft: BatchDraft,
  keys: readonly KeyStatus[],
  problem: string | null
): string | null {
  if (draft.items.length === 0) return 'Select at least one asset.';
  if (draft.items.length > MAX_BATCH) return `A batch takes at most ${MAX_BATCH} assets.`;
  const bare = draft.items.find(i => i.images.length === 0);
  if (bare) return `${bare.name} has no refs yet. Send a pick to Refs first.`;
  if (draft.provider !== 'by-hand' && !keys.find(k => k.service === draft.provider)?.saved)
    return `Add a ${PROVIDER_LABEL[draft.provider]} API key in Settings > 3D providers first.`;
  return problem;
}

const count = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** "6 models · 5 props, 1 character · By hand, Tripo settings". */
export function batchSummary(draft: BatchDraft): string {
  const props = draft.items.filter(i => i.variant === 'prop').length;
  const characters = draft.items.length - props;
  const kinds = [
    props ? count(props, 'prop') : null,
    characters ? count(characters, 'character') : null,
  ]
    .filter(Boolean)
    .join(', ');
  const where =
    draft.provider === 'by-hand'
      ? `By hand, ${SERVICE_LABEL[draft.service]} settings`
      : PROVIDER_LABEL[draft.provider];
  return [count(draft.items.length, 'model'), kinds, where].filter(Boolean).join(' · ');
}

/** The footer button: "Write 6 handoff folders", "Send 6 to Tripo". */
export function batchLabel(draft: BatchDraft): string {
  const n = draft.items.length;
  return draft.provider === 'by-hand'
    ? `Write ${count(n, 'handoff folder')}`
    : `Send ${n} to ${PROVIDER_LABEL[draft.provider]}`;
}

export interface ChecklistGroup {
  /** The batch id, or null for single sends. */
  batch: string | null;
  jobs: ProviderJob[];
  /** How many models came back. */
  back: number;
  createdAtMs: number;
}

const BACK: ReadonlySet<ProviderJob['state']> = new Set(['inbox', 'accepted']);

/**
 * Every By hand send, grouped by batch, newest group first. A group shows
 * while any of its sends still waits or none was cancelled.
 */
export function checklist(jobs: readonly ProviderJob[]): ChecklistGroup[] {
  const groups = new Map<string, ChecklistGroup>();
  for (const job of jobs) {
    if (job.provider !== 'by-hand') continue;
    const key = job.batch ?? `single:${job.id}`;
    const group = groups.get(key) ?? {
      batch: job.batch,
      jobs: [],
      back: 0,
      createdAtMs: job.createdAtMs ?? 0,
    };
    group.jobs.push(job);
    if (BACK.has(job.state)) group.back += 1;
    group.createdAtMs = Math.max(group.createdAtMs, job.createdAtMs ?? 0);
    groups.set(key, group);
  }
  return [...groups.values()]
    .filter(g => g.jobs.some(j => j.state !== 'cancelled'))
    .map(g => ({ ...g, jobs: [...g.jobs].sort((a, b) => a.assetName.localeCompare(b.assetName)) }))
    .sort((a, b) => b.createdAtMs - a.createdAtMs);
}

/** True when the job still wants a model: the checklist row offers Add model. */
export function wantsModel(job: ProviderJob): boolean {
  return job.provider === 'by-hand' && job.state === 'waiting';
}

/** By hand sends still waiting for a model. */
export function waitingByHand(jobs: readonly ProviderJob[]): ProviderJob[] {
  return jobs.filter(wantsModel);
}

/** "4 of 6 back", "All 6 back". */
export function backLabel(group: Pick<ChecklistGroup, 'jobs' | 'back'>): string {
  const live = group.jobs.filter(j => j.state !== 'cancelled').length;
  return group.back >= live ? `All ${live} back` : `${group.back} of ${live} back`;
}

/** The first match for every file of a scan: its own, or none. */
export function initialMatches(scan: DownloadsScan): Record<string, string | null> {
  const out: Record<string, string | null> = {};
  for (const file of scan.files) out[file.path] = file.job;
  return out;
}
