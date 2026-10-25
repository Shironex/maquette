import type { SendOptions } from '@/ipc/client';
import {
  isSmartTopology,
  meshyProblem,
  meshySummary,
  tripoProblem,
  tripoSummary,
} from '@/state/provider-presets';
import { PROVIDER_LABEL, sendBlocker, sortJobs, type SendDraft } from '@/state/providers';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useCatalog } from '../../PresetFields';

/** The draft's actions and what the form derives from it: blocker, summary, key, earlier sends. */
export function useSendForm(options: SendOptions, draft: SendDraft) {
  const setDraft = useProvidersStore(s => s.setDraft);
  const reset = useProvidersStore(s => s.resetDraft);
  const send = useProvidersStore(s => s.send);
  const sending = useProvidersStore(s => s.sending);
  const close = useProvidersStore(s => s.closePanel);
  const keys = useProvidersStore(s => s.keys);
  const allJobs = useProvidersStore(s => s.jobs);
  const setView = useAppStore(s => s.setView);
  const catalog = useCatalog();
  const service = draft.service;
  const problem =
    service === 'tripo'
      ? tripoProblem(draft.tripo, catalog.tripoModels)
      : meshyProblem(draft.meshy, catalog.meshyModels);
  const blocker = sendBlocker(draft.provider, keys, draft.images, problem);
  const multiView = service === 'tripo' ? draft.tripo.multiView : draft.meshy.multiView;
  const smart = service === 'meshy' && isSmartTopology(draft.meshy.aiModel);
  const summary =
    service === 'tripo'
      ? tripoSummary(draft.tripo, catalog.tripoModels)
      : meshySummary(draft.meshy, catalog.meshyModels);
  const key = keys.find(k => k.service === draft.provider);
  const earlier = sortJobs(allJobs.filter(j => j.asset === options.asset.id)).slice(0, 4);
  const label =
    draft.provider === 'by-hand'
      ? 'Write the handoff folder'
      : `Send to ${PROVIDER_LABEL[draft.provider]}`;
  const openSettings = () => {
    close();
    setView('settings');
  };
  return {
    setDraft,
    reset,
    send,
    sending,
    close,
    openSettings,
    blocker,
    multiView,
    smart,
    summary,
    key,
    earlier,
    label,
  };
}
