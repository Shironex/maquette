/** Words and numbers for Send to 3D: job lines, the Board card state, the panel. */
import type { IconName, Status } from '@maquette/ui';
import type {
  Asset,
  JobPhase,
  JobState,
  KeyStatus,
  Provider,
  ProviderId,
  ProviderJob,
  TripoPreset,
  MeshyPreset,
} from '@/ipc/client';

export const PROVIDER_LABEL: Record<Provider, string> = {
  tripo: 'Tripo',
  meshy: 'Meshy',
  'by-hand': 'By hand',
};

export const SERVICE_LABEL: Record<ProviderId, string> = { tripo: 'Tripo', meshy: 'Meshy' };

const PHASE_LABEL: Record<JobPhase, string> = {
  upload: 'Uploading',
  generate: 'Generating',
  'rig-check': 'Checking the rig',
  rig: 'Rigging',
  animate: 'Animating',
  download: 'Downloading',
};

/** The most images a send takes; Tripo multi-view and Meshy both stop at 4. */
export const MAX_IMAGES = 4;

/** Multi-view views in the order Maquette sends them. */
export const VIEW_NAMES = ['Front', 'Left', 'Back', 'Right'] as const;

export function isActive(state: JobState): boolean {
  return state === 'approval' || state === 'queued' || state === 'running' || state === 'waiting';
}

/** Newest first, active jobs ahead of finished ones. */
export function sortJobs(jobs: readonly ProviderJob[]): ProviderJob[] {
  return [...jobs].sort(
    (a, b) =>
      Number(isActive(b.state)) - Number(isActive(a.state)) ||
      (b.createdAtMs ?? 0) - (a.createdAtMs ?? 0)
  );
}

/** The asset's newest send, if any. */
export function latestJob(jobs: readonly ProviderJob[], asset: string): ProviderJob | undefined {
  let found: ProviderJob | undefined;
  for (const job of jobs) {
    if (job.asset !== asset) continue;
    if (!found || (job.createdAtMs ?? 0) > (found.createdAtMs ?? 0)) found = job;
  }
  return found;
}

/** 0 to 100, for the progress bar. */
export function percent(job: ProviderJob): number {
  return Math.round(Math.min(1, Math.max(0, job.progress ?? 0)) * 100);
}

export interface JobChip {
  status: Status;
  label: string;
  icon?: IconName;
}

/** The status chip a job shows, in the queue and on the Board card. */
export function jobChip(job: ProviderJob): JobChip {
  switch (job.state) {
    case 'approval':
      return { status: 'queued', label: 'Waiting for approval', icon: 'robot' };
    case 'queued':
      return { status: 'queued', label: 'Queued' };
    case 'running':
      return { status: 'running', label: `${percent(job)}%` };
    case 'waiting':
      return { status: 'queued', label: 'Waiting for your model', icon: 'hand-pointing' };
    case 'inbox':
      return { status: 'review', label: 'In the Inbox', icon: 'tray' };
    case 'accepted':
      return { status: 'done', label: job.version ? `Accepted as v${job.version}` : 'Accepted' };
    case 'failed':
      return { status: 'failed', label: 'Failed' };
    case 'cancelled':
      return { status: 'cached', label: 'Cancelled', icon: 'x' };
  }
}

/** "Tripo, generating", "By hand, Tripo settings". */
export function jobTitle(job: ProviderJob): string {
  if (job.provider === 'by-hand') return `By hand, ${SERVICE_LABEL[job.service]} settings`;
  const phase = job.state === 'running' && job.phase ? PHASE_LABEL[job.phase].toLowerCase() : null;
  return phase ? `${PROVIDER_LABEL[job.provider]}, ${phase}` : PROVIDER_LABEL[job.provider];
}

/** "12 credits", or null before the provider reports any. */
export function creditsText(job: ProviderJob): string | null {
  if (job.provider === 'by-hand') return 'no credits';
  const n = job.credits;
  if (n === null || n === undefined) return null;
  const shown = Number.isInteger(n) ? String(n) : n.toFixed(1);
  return `${shown} ${n === 1 ? 'credit' : 'credits'}`;
}

/** "42s", "1m 12s", "2h 5m", "3d". */
export function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ${m % 60}m`;
  return `${Math.floor(h / 24)}d`;
}

/** How long the job has run, or ran. */
export function elapsedText(job: ProviderJob, now: number): string | null {
  const start = job.startedAtMs ?? (job.state === 'waiting' ? job.createdAtMs : null);
  if (!start) return null;
  const end = job.finishedAtMs ?? now;
  return formatElapsed(Math.max(0, end - start));
}

/** The queue's meta line: "2 images · 1m 12s · 30 credits". */
export function jobMeta(job: ProviderJob, now: number): string {
  const images = `${job.images.length} ${job.images.length === 1 ? 'image' : 'images'}`;
  return [
    images,
    elapsedText(job, now),
    creditsText(job),
    job.viaAgent ? 'asked by an agent' : null,
  ]
    .filter(Boolean)
    .join(' · ');
}

export interface CardState {
  status: Status;
  label: string;
  icon?: IconName;
  /** 0 to 100 while a provider works on it. */
  progress?: number;
  progressLabel?: string;
}

/**
 * What the Board card shows for an asset with a send under way, or null
 * to keep the asset's own status.
 */
export function cardState(job: ProviderJob | undefined): CardState | null {
  if (!job) return null;
  const chip = jobChip(job);
  if (job.state === 'running') {
    return { ...chip, progress: percent(job), progressLabel: jobTitle(job) };
  }
  if (job.state === 'failed') return { ...chip, label: `${PROVIDER_LABEL[job.provider]} failed` };
  if (job.state === 'queued')
    return { ...chip, label: `Queued for ${PROVIDER_LABEL[job.provider]}` };
  if (isActive(job.state) || job.state === 'inbox') return chip;
  return null;
}

/** Why the panel cannot send yet, or null. */
export function sendBlocker(
  provider: Provider,
  keys: readonly KeyStatus[],
  images: readonly string[],
  problem: string | null
): string | null {
  if (images.length === 0) return 'Pick at least one image.';
  if (provider !== 'by-hand') {
    const saved = keys.find(k => k.service === provider)?.saved;
    if (!saved)
      return `Add a ${PROVIDER_LABEL[provider]} API key in Settings > 3D providers first.`;
  }
  return problem;
}

/** What the panel says the images become with these settings. */
export function imagesNote(
  provider: Provider,
  service: ProviderId,
  images: readonly string[],
  multiView: boolean,
  smartTopology: boolean
): string {
  if (images.length === 0) return 'Pick up to 4 images, front first.';
  if (provider === 'by-hand') {
    return images.length === 1
      ? 'The handoff folder gets this image.'
      : `The handoff folder gets ${images.length} images, named by view.`;
  }
  if (images.length === 1) return 'One image to one model.';
  if (service === 'meshy' && smartTopology) {
    return 'Smart Topology models take one image, so only the first is sent.';
  }
  if (!multiView) return 'Multi-view is off, so only the first image is sent.';
  return service === 'tripo'
    ? `${images.length} views go to Tripo's multi-view model, front first.`
    : `${images.length} images go to Meshy's multi-image model, front first.`;
}

/** The kind of preset an asset starts from. */
export function variantFor(asset: Pick<Asset, 'kind'>): 'prop' | 'character' {
  return asset.kind === 'character' ? 'character' : 'prop';
}

/** The settings a send uses, for the request. */
export interface SendDraft {
  provider: Provider;
  service: ProviderId;
  variant: 'prop' | 'character';
  images: string[];
  tripo: TripoPreset;
  meshy: MeshyPreset;
  /** Meshy only: text that guides the texture. */
  prompt?: string;
}

/** Meshy refuses a texture prompt longer than this. */
export const MAX_PROMPT = 800;

/** Why the texture prompt cannot go, or null. */
export function promptProblem(service: ProviderId, prompt: string | undefined): string | null {
  if (service !== 'meshy' || (prompt?.trim().length ?? 0) <= MAX_PROMPT) return null;
  return `Keep the texture prompt to ${MAX_PROMPT} characters; Meshy refuses longer ones.`;
}

/** The prompt a request carries: Meshy only, trimmed, null when blank. */
export function promptFor(service: ProviderId, prompt: string | undefined): string | null {
  if (service !== 'meshy') return null;
  const text = prompt?.trim();
  return text ? text : null;
}

/** Turns an image on or off, keeping the order and at most 4. */
export function toggleImage(images: readonly string[], file: string): string[] {
  if (images.includes(file)) return images.filter(i => i !== file);
  if (images.length >= MAX_IMAGES) return [...images];
  return [...images, file];
}

/** Moves an image to the front. */
export function makeFront(images: readonly string[], file: string): string[] {
  return images.includes(file) ? [file, ...images.filter(i => i !== file)] : [...images];
}
