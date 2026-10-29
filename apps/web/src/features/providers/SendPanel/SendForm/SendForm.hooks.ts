import { commands, type RefFix, type RefWarning, type SendOptions } from '@/ipc/client';
import { useAsyncValue } from '@/lib/useAsyncValue';
import {
  isSmartTopology,
  meshyProblem,
  meshySummary,
  tripoProblem,
  tripoSummary,
} from '@/state/provider-presets';
import {
  PROVIDER_LABEL,
  promptProblem,
  sendBlocker,
  sortJobs,
  type SendDraft,
} from '@/state/providers';
import { sendRequest } from '@/state/providers-store-helpers';
import { redoLine } from '@/state/redo';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useCatalog } from '../../PresetFields';
import { splitModelNote } from '@/state/refs';
import { useRefWarnings, useWarningFix } from '../../RefWarnings';

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
  const blocker = sendBlocker(
    draft.provider,
    keys,
    draft.images,
    problem ?? promptProblem(service, draft.prompt)
  );
  const multiView = service === 'tripo' ? draft.tripo.multiView : draft.meshy.multiView;
  const smart = service === 'meshy' && isSmartTopology(draft.meshy.aiModel);
  const summary =
    service === 'tripo'
      ? tripoSummary(draft.tripo, catalog.tripoModels)
      : meshySummary(draft.meshy, catalog.meshyModels);
  const key = keys.find(k => k.service === draft.provider);
  const earlier = sortJobs(allJobs.filter(j => j.asset === options.asset.id)).slice(0, 4);
  const redo = useProvidersStore(s => s.panel?.redo ?? null);
  const label =
    draft.provider === 'by-hand'
      ? 'Write the handoff folder'
      : `${redo ? 'Redo with' : 'Send to'} ${PROVIDER_LABEL[draft.provider]}`;
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
    redo: redo ? { line: redoLine(redo), name: redo.assetName } : null,
  };
}

/** What the send should cost with the draft as it stands, from the price lists. */
export function useSendEstimate(options: SendOptions, draft: SendDraft) {
  const redo = useProvidersStore(s => s.panel?.redo ?? null);
  const request =
    draft.provider === 'by-hand'
      ? null
      : sendRequest({ project: options.project, asset: options.asset.id, redo }, draft);
  return useAsyncValue(commands.providerEstimate, request);
}

/** The weak ref warnings of the images as picked, their fixes, and the existing model note. */
export function useSendWarnings(options: SendOptions, draft: SendDraft, multiView: boolean) {
  const setDraft = useProvidersStore(s => s.setDraft);
  const items = [{ asset: options.asset.id, images: draft.images, multiView }];
  const all = useRefWarnings(options.project, draft.images.length > 0 ? items : [])?.[0] ?? [];
  const { weak: warnings, note } = splitModelNote(all);
  const fix = useWarningFix(options.project, (_, file) =>
    setDraft({ images: draft.images.filter(i => i !== file) })
  );
  return { warnings, note, fix: (w: RefWarning, f: RefFix) => fix(options.asset.id, w, f) };
}
