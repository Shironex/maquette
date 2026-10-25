import { useState } from 'react';
import type { ProviderJob } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** The row's asset thumbnail, its folder action and whether its card is open. */
export function useChecklistRow(job: ProviderJob) {
  const workspace = useAppStore(s => s.workspace);
  const openHandoff = useProvidersStore(s => s.openHandoff);
  const [expanded, setExpanded] = useState(false);
  const asset = workspace?.project?.assets?.find(a => a.id === job.asset);
  const thumb = fileUrl(
    asset?.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null
  );
  const toggleExpanded = () => setExpanded(v => !v);
  return { asset, thumb, openHandoff, expanded, toggleExpanded };
}
