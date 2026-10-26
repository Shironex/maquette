import type { InspectModel } from '@/ipc/client';
import { updateFor } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';

/** The model's asset, the library entry it came from and an update waiting there. */
export function useLibraryPanel(project: string, model: InspectModel) {
  const id = model.tweakAsset;
  const asset = useAppStore(s =>
    s.workspace?.projects.find(p => p.id === project)?.assets?.find(a => a.id === id)
  );
  const view = useLibraryStore(s => s.view);
  const openAdd = useLibraryStore(s => s.openAdd);
  const openUpdate = useLibraryStore(s => s.openUpdate);
  return {
    asset,
    update: asset ? updateFor(view, asset) : null,
    entry: view?.items.find(i => i.entry.id === asset?.origin?.entry)?.entry ?? null,
    openAdd,
    openUpdate,
  };
}
