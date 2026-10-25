import { useState } from 'react';
import type { PresetVariant, ProviderId } from '@/ipc/client';

/** Which saved preset is in view: a service and a variant. */
export function usePresetPick() {
  const [service, setService] = useState<ProviderId>('tripo');
  const [variant, setVariant] = useState<PresetVariant>('prop');
  return { service, setService, variant, setVariant };
}
