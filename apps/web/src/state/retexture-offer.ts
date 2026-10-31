/**
 * A split accepted with "Re-texture with Meshy" chosen: nothing goes out on
 * its own, because the credits need a person's word. The accept result says
 * so and links to the offer in the Parts tab.
 */
import type { InboxItem } from '@/ipc/client';
import { useAppStore, type Notice } from './use-app-store';
import { useInspectorStore } from './use-inspector-store';
import { useProvidersStore } from './use-providers-store';

/** Shows the accepted version in the Inspector's Parts tab, where the offer sits. */
export function openRetextureOffer(project: string, asset: string, version: number): void {
  const inspector = useInspectorStore.getState();
  inspector.focusVersion(project, asset, version);
  inspector.setTab('parts');
  useAppStore.getState().setView('inspector');
}

/** The words and the link for an accepted split that waits for a re-texture; null otherwise. */
export function retextureOffer(
  item: InboxItem | null | undefined,
  version: number
): Pick<Notice, 'detail' | 'action'> | null {
  const link = item?.link;
  if (!link) return null;
  const job = useProvidersStore.getState().jobs.find(j => j.id === link.job);
  if (job?.split?.after !== 'retexture') return null;
  return {
    detail: 'Your re-texture is not sent yet. It waits in the Parts tab with your prompt.',
    action: {
      label: 'Open Parts tab',
      run: () => openRetextureOffer(link.project, link.asset, version),
    },
  };
}
