import { fileUrl } from '@/ipc/files';
import type { Arrival } from '@/ipc/client';
import { projectFile } from '@/state/board';
import { otherSuggestions, shownSuggestion, suggestionLabel } from '@/state/downloads';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/**
 * The download's front render next to the front ref of the send it looks
 * like, how alike they are, the other waiting sends ranked after it, and
 * the answers to a look.
 */
export function useLookCard(arrival: Arrival) {
  const workspace = useAppStore(s => s.workspace);
  const send = useDownloadsStore(s => s.status?.pending.find(p => p.job === arrival.job));
  const busy = useDownloadsStore(s => s.busy.includes(arrival.id));
  const attach = useDownloadsStore(s => s.attach);
  const openChooser = useDownloadsStore(s => s.openChooser);
  const dismiss = useDownloadsStore(s => s.dismiss);
  const project = workspace?.project;
  const shown = shownSuggestion(arrival);
  const asset = project?.assets?.find(a => a.id === arrival.asset);
  const picture = shown?.image ?? send?.image ?? asset?.thumbnail ?? null;
  const here = Boolean(arrival.project && arrival.project === project?.id);
  const others = otherSuggestions(arrival).map(s => ({
    job: s.job,
    label: `${s.assetName} · ${suggestionLabel(s)}`,
    here: s.project === project?.id,
    attach: () => void attach(arrival.id, s.asset),
  }));
  return {
    ref: fileUrl(picture && workspace && here ? projectFile(workspace, picture) : null),
    render: fileUrl(arrival.render ?? null),
    fit: shown && shown.job === arrival.job ? suggestionLabel(shown) : null,
    others,
    here,
    busy,
    attach: () => {
      if (arrival.asset) void attach(arrival.id, arrival.asset);
    },
    pickAnother: () => openChooser(arrival.id),
    notThis: () => void dismiss(arrival.id),
  };
}
