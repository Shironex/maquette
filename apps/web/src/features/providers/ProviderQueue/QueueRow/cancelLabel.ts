import type { ProviderJob } from '@/ipc/client';

export const CANCEL_LABEL: Record<ProviderJob['cancel'], string | null> = {
  free: 'Cancel',
  provider: 'Cancel at Meshy',
  'stop-waiting': 'Stop waiting',
  none: null,
};
