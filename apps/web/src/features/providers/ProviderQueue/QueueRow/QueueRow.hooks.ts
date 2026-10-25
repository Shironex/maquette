import type { ProviderJob } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** The asset a send is for, its thumbnail and the row's actions. */
export function useQueueRow(job: ProviderJob) {
  const workspace = useAppStore(s => s.workspace);
  const asset = workspace?.project?.assets?.find(a => a.id === job.asset);
  const cancel = useProvidersStore(s => s.cancel);
  const showHandoff = useProvidersStore(s => s.showHandoff);
  const busy = useProvidersStore(s => s.busy.includes(job.id));
  const thumbPath = asset?.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;
  const thumb = fileUrl(thumbPath);
  return { asset, thumb, cancel, showHandoff, busy };
}
