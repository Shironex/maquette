/**
 * Redo from the Inbox: a send panel or a batch prefilled from the send an
 * inbox model came from, so the person only changes what should differ.
 * The redo goes out as the asset's next version; the earlier model stays in
 * the inbox until one of them is accepted.
 */
import type {
  InboxItem,
  MeshyPreset,
  PresetVariant,
  ProviderJob,
  SendOptions,
  TripoPreset,
} from '@/ipc/client';
import { presetChanges, studioFor, type BatchDraft } from './batch';
import { MAX_IMAGES, PROVIDER_LABEL, isActive, type SendDraft } from './providers';

/** Why an inbox model cannot be redone, or null. */
export function redoBlocker(item: Pick<InboxItem, 'link'>): string | null {
  if (!item.link) return 'It did not come from a Send to 3D, so there are no settings to repeat';
  return null;
}

/**
 * The send panel's draft for a redo of `job`: its provider, kind, images
 * and settings, laid over `fresh` (the saved preset for its kind). Images
 * gone from refs/ are dropped; when none are left the asset's own refs are
 * picked again.
 */
export function redoDraft(job: ProviderJob, options: SendOptions, fresh: SendDraft): SendDraft {
  const known = new Set(options.images.map(i => i.file));
  const images = job.images.filter(i => known.has(i)).slice(0, MAX_IMAGES);
  return {
    provider: job.provider,
    service: job.service,
    variant: job.variant,
    images: images.length > 0 ? images : fresh.images,
    tripo: job.tripo ? structuredClone(job.tripo) : fresh.tripo,
    meshy: job.meshy ? structuredClone(job.meshy) : fresh.meshy,
    prompt: job.prompt ?? '',
    studio: job.studio ?? fresh.studio,
  };
}

/** The earlier send of each asset of a batch redo, by asset id. */
export type RedoSources = Record<string, ProviderJob>;

/** The send most of `jobs` went to, ties to the first. */
function mostUsed(jobs: readonly ProviderJob[]): ProviderJob | undefined {
  const count = (j: ProviderJob) => jobs.filter(o => o.provider === j.provider).length;
  return [...jobs].sort((a, b) => count(b) - count(a))[0];
}

/**
 * A batch draft for redoing several inbox models: the provider most of them
 * went to, each kind's batch preset from the first send of that kind, and
 * each asset's own differences from it kept as its custom settings, with
 * its images, texture prompt and the send it redoes.
 */
export function redoBatchDraft(base: BatchDraft, sources: RedoSources): BatchDraft {
  const jobs = base.items.map(i => sources[i.asset]).filter((j): j is ProviderJob => Boolean(j));
  const lead = mostUsed(jobs);
  if (!lead) return base;
  const service = lead.service;
  const tripo = structuredClone(base.tripo);
  const meshy = structuredClone(base.meshy);
  for (const variant of ['prop', 'character'] as const) {
    const first = jobs.find(j => j.variant === variant && j.service === service);
    if (first?.tripo) tripo[variant] = structuredClone(first.tripo);
    if (first?.meshy) meshy[variant] = structuredClone(first.meshy);
  }
  const own = <T extends object>(
    preset: T | null,
    batch: Record<PresetVariant, T>,
    v: PresetVariant
  ) => (preset ? presetChanges(batch[v], preset) : {});
  return {
    ...base,
    provider: lead.provider,
    service,
    tripo,
    meshy,
    studio: studioFor(tripo),
    items: base.items.map(item => {
      const job = sources[item.asset];
      if (!job) return item;
      return {
        ...item,
        variant: job.variant,
        images: job.images.length > 0 ? job.images.slice(0, MAX_IMAGES) : item.images,
        tripo: job.service === service ? own<TripoPreset>(job.tripo, tripo, job.variant) : {},
        meshy: job.service === service ? own<MeshyPreset>(job.meshy, meshy, job.variant) : {},
        studio: job.studio ? presetChanges(studioFor(tripo)[job.variant], job.studio) : {},
        prompt: job.prompt ?? '',
        redoOf: job.id,
      };
    }),
  };
}

/** A redo of the inbox model's send still under way, if any. */
export function redoUnderWay(
  jobs: readonly ProviderJob[],
  item: Pick<InboxItem, 'link'>
): ProviderJob | undefined {
  const of = item.link?.job;
  if (!of) return undefined;
  return jobs.find(j => j.redoOf === of && isActive(j.state));
}

/** "Redo of veil-gate-1, Tripo": the panel's line about what it repeats. */
export function redoLine(job: Pick<ProviderJob, 'id' | 'provider'>): string {
  return `Redo of ${job.id}, sent to ${PROVIDER_LABEL[job.provider]}`;
}

export type BatchRedoPlan =
  | { project: string; redo: { asset: string; job: string }[]; blocker: null }
  | { project: null; redo: []; blocker: string };

const blocked = (blocker: string): BatchRedoPlan => ({ project: null, redo: [], blocker });

/**
 * What a batch redo of the checked inbox models sends, or why it cannot:
 * each must have come from a send, all from one project, one model per asset.
 */
export function batchRedoPlan(
  items: readonly InboxItem[],
  checked: readonly string[]
): BatchRedoPlan {
  const picked = items.filter(i => checked.includes(i.id));
  if (picked.length === 0) return blocked('Select the models to redo');
  const bare = picked.find(i => !i.link);
  if (bare) return blocked(`${bare.name} did not come from a Send to 3D`);
  const links = picked.map(i => i.link).filter((l): l is NonNullable<InboxItem['link']> => !!l);
  const [first] = links;
  if (!first) return blocked('Select the models to redo');
  if (links.some(l => l.project !== first.project))
    return blocked('They were sent from different projects; redo each project on its own');
  const seen = new Set<string>();
  for (const l of links) {
    if (seen.has(l.asset))
      return blocked(`Two of them are ${l.assetName}; a batch takes one per asset`);
    seen.add(l.asset);
  }
  return {
    project: first.project,
    redo: links.map(l => ({ asset: l.asset, job: l.job })),
    blocker: null,
  };
}
