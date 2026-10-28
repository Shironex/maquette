/**
 * Words for the Downloads watcher: the quiet notice a new model gets, the
 * downloads waiting for a look or asking "Which asset is this?", the assets
 * they can be attached to and the line Settings shows about the watch.
 */
import type { ToastStatus } from '@maquette/ui';
import type { Arrival, Asset, PendingHandoff, Stage, WatchStatus } from '@/ipc/client';
import { formatAgo, formatBytes } from '@/lib/format';
import { SERVICE_LABEL } from './providers';

export interface ArrivalNotice {
  status: ToastStatus;
  title: string;
  detail: string;
  /** True when the notice should offer the picker. */
  ask: boolean;
  /** The picker button's label, when it asks. */
  action?: string;
}

/** What to say when the watcher took a look at `arrival`; null to say nothing. */
export function arrivalNotice(arrival: Arrival): ArrivalNotice | null {
  const asset = arrival.assetName ?? 'its asset';
  switch (arrival.state) {
    case 'imported':
      return {
        status: 'done',
        title: `${asset} is in the Inbox`,
        detail: `${arrival.name} came in from Downloads. The checks run now; the original stays where it was.`,
        ask: false,
      };
    case 'review':
      return {
        status: 'info',
        title: `Is this ${asset}?`,
        detail: `${arrival.name} needs a look before it goes in. ${arrival.reason}.`,
        ask: true,
        action: 'Look',
      };
    case 'unmatched':
      return {
        status: 'info',
        title: 'A new model in Downloads',
        detail: `${arrival.name}: which asset is this? ${arrival.reason}.`,
        ask: true,
        action: 'Choose',
      };
    case 'attached':
      return {
        status: 'done',
        title: `${arrival.name} is in the Inbox for ${asset}`,
        detail: 'The checks run now; accept it there. The original stays where it was.',
        ask: false,
      };
    case 'needs-glb':
      return {
        status: 'info',
        title: `${arrival.name} is an FBX`,
        detail: arrival.reason,
        ask: false,
      };
    case 'failed':
      return {
        status: 'failed',
        title: `Could not use ${arrival.name}`,
        detail: arrival.reason,
        ask: false,
      };
    case 'dismissed':
      return null;
  }
}

/** Downloads whose name only looks like a waiting send's, newest first. */
export function lookingArrivals(status: WatchStatus | null): Arrival[] {
  return (status?.arrivals ?? []).filter(a => a.state === 'review');
}

/** Downloads asking which asset they are, newest first. */
export function askingArrivals(status: WatchStatus | null): Arrival[] {
  return (status?.arrivals ?? []).filter(a => a.state === 'unmatched' || a.state === 'failed');
}

/** Every download waiting for an answer: the ones to look at first. */
export function waitingArrivals(status: WatchStatus | null): Arrival[] {
  return [...lookingArrivals(status), ...askingArrivals(status)];
}

/** "holds model.glb · 4.2 MB · 2 min ago" for a download. */
export function arrivalMeta(arrival: Arrival, now: number): string {
  return [
    arrival.inner ? `holds ${arrival.inner}` : null,
    arrival.bytes !== null ? formatBytes(arrival.bytes) : null,
    arrival.atMs !== null ? formatAgo(arrival.atMs, now) : null,
  ]
    .filter(Boolean)
    .join(' · ');
}

/** One asset a download can be attached to. */
export interface AssetChoice {
  asset: string;
  name: string;
  /** Its picture, relative to the project folder: the ref it was sent with, else its own. */
  picture: string | null;
  /** True while a By hand send of it waits for a model. */
  waiting: boolean;
  /** Where it stands, in a few words. */
  state: string;
}

/**
 * The project's assets a download can go to, the ones a By hand send
 * waits on first, narrowed to the names that hold every word of `query`.
 */
export function assetChoices(
  assets: readonly Asset[],
  pending: readonly PendingHandoff[],
  project: string,
  stages: Record<Stage, string>,
  query = ''
): AssetChoice[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const choices = assets.map((asset): AssetChoice => {
    const send = pending.find(p => p.project === project && p.asset === asset.id);
    const n = asset.versions.length;
    return {
      asset: asset.id,
      name: asset.name,
      picture: send?.image ?? asset.thumbnail ?? null,
      waiting: Boolean(send),
      state: send
        ? `Waiting for its ${SERVICE_LABEL[send.service]} model`
        : n === 0
          ? `${stages[asset.stage]}, no model yet`
          : `${stages[asset.stage]}, v${n}`,
    };
  });
  return choices
    .filter(c => words.every(w => c.name.toLowerCase().includes(w)))
    .sort((a, b) => Number(b.waiting) - Number(a.waiting));
}

/** The line under the switch in Settings. */
export function watchLine(status: WatchStatus | null): {
  text: string;
  tone: 'ok' | 'mute' | 'error';
} {
  if (!status) return { text: 'Reading the watcher', tone: 'mute' };
  if (!status.enabled)
    return { text: 'Off. Use Add model or Check Downloads instead.', tone: 'mute' };
  if (status.error) return { text: status.error, tone: 'error' };
  if (!status.watching) return { text: 'Not watching yet', tone: 'mute' };
  const n = status.pending.length;
  return {
    text:
      n === 0
        ? 'Watching. Nothing waits for a model, so nothing is read.'
        : `Watching for ${n} By hand ${n === 1 ? 'send' : 'sends'}.`,
    tone: 'ok',
  };
}
