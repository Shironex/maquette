import type { ProviderId } from '@/ipc/client';
import { SERVICE_LABEL } from '@/state/providers';

/** "Open Tripo studio", "Open Meshy". */
export function siteLabel(service: ProviderId): string {
  return service === 'tripo' ? 'Open Tripo studio' : `Open ${SERVICE_LABEL[service]}`;
}
