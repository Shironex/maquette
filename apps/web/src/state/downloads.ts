/**
 * Words for the Downloads watcher: the quiet notice a new model gets, the
 * downloads still asking "Which asset is this?", the sends they can go to
 * and the line Settings shows about the watch.
 */
import type { ToastStatus } from '@maquette/ui';
import type { Arrival, PendingHandoff, WatchStatus } from '@/ipc/client';

export interface ArrivalNotice {
  status: ToastStatus;
  title: string;
  detail: string;
  /** True when the notice should offer the "Which asset is this?" picker. */
  ask: boolean;
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
    case 'unmatched':
      return {
        status: 'info',
        title: 'A new model in Downloads',
        detail: `${arrival.name}: which asset is this? ${arrival.reason}.`,
        ask: true,
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

/** Downloads waiting for an answer, newest first. */
export function askingArrivals(status: WatchStatus | null): Arrival[] {
  return (status?.arrivals ?? []).filter(a => a.state === 'unmatched' || a.state === 'failed');
}

/** The sends a download can go to, named by asset, and by project when there are several. */
export function sendOptions(
  pending: readonly PendingHandoff[],
  projectName: (id: string) => string
): { value: string; label: string }[] {
  const several = new Set(pending.map(p => p.project)).size > 1;
  return pending.map(p => ({
    value: p.job,
    label: several ? `${p.assetName} (${projectName(p.project)})` : p.assetName,
  }));
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
