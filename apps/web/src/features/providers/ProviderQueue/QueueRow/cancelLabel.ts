import type { ProviderJob } from '@/ipc/client';

/** What each way of cancelling does, as its tooltip. */
export const CANCEL_HINT: Record<ProviderJob['cancel'], string | null> = {
  free: 'Stops this send before it goes any further',
  provider: 'Asks Meshy to stop this task',
  'stop-waiting': 'Stops waiting for this send; the provider finishes it and it lands in the Inbox',
  none: null,
};

export const CANCEL_LABEL: Record<ProviderJob['cancel'], string | null> = {
  free: 'Cancel',
  provider: 'Cancel at Meshy',
  'stop-waiting': 'Stop waiting',
  none: null,
};
