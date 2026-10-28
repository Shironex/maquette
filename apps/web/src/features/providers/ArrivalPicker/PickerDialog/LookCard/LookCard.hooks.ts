import { fileUrl } from '@/ipc/files';
import type { Arrival } from '@/ipc/client';
import { projectFile } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The ref the suggested send went out with, and the three answers to a look. */
export function useLookCard(arrival: Arrival) {
  const workspace = useAppStore(s => s.workspace);
  const send = useDownloadsStore(s => s.status?.pending.find(p => p.job === arrival.job));
  const busy = useDownloadsStore(s => s.busy.includes(arrival.id));
  const attach = useDownloadsStore(s => s.attach);
  const openChooser = useDownloadsStore(s => s.openChooser);
  const dismiss = useDownloadsStore(s => s.dismiss);
  const asset = workspace?.project?.assets?.find(a => a.id === arrival.asset);
  const picture = send?.image ?? asset?.thumbnail ?? null;
  const here = Boolean(arrival.project && arrival.project === workspace?.project?.id);
  return {
    ref: fileUrl(picture && workspace && here ? projectFile(workspace, picture) : null),
    here,
    busy,
    attach: () => {
      if (arrival.asset) void attach(arrival.id, arrival.asset);
    },
    pickAnother: () => openChooser(arrival.id),
    notThis: () => void dismiss(arrival.id),
  };
}
