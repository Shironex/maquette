import { useKeyDraft } from '../KeyRowView';
import type { ProviderId } from '@/ipc/client';
import { useProvidersStore } from '@/state/use-providers-store';

/** One service's saved key, the draft for a new one and the last test result. */
export function useKeyRow(service: ProviderId) {
  const status = useProvidersStore(s => s.keys.find(k => k.service === service));
  const busy = useProvidersStore(s => s.keyBusy === service);
  const save = useProvidersStore(s => s.saveKey);
  const remove = useProvidersStore(s => s.removeKey);
  const test = useProvidersStore(s => s.testKey);
  return useKeyDraft({
    status,
    busy,
    save: key => save(service, key),
    remove: () => remove(service),
    test: key => test(service, key),
  });
}
