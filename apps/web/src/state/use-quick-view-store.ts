/** The 3D quick view the Board's card menu and the Library's pictures open. */
import { create } from 'zustand';
import type { LibraryItem } from '@/ipc/client';
import { libraryQuickView, type InspectTarget, type QuickViewTarget } from './quick-view';
import { useAppStore } from './use-app-store';
import { useInspectorStore } from './use-inspector-store';

interface QuickViewStore {
  /** The model on show, or null while the quick view is closed. */
  target: QuickViewTarget | null;
  open: (target: QuickViewTarget) => void;
  close: () => void;
  /** Closes the quick view and shows the version in the Inspector's compare tab. */
  openInspector: (inspect: InspectTarget) => void;
}

export const useQuickViewStore = create<QuickViewStore>(set => ({
  target: null,
  open: target => set({ target }),
  close: () => set({ target: null }),
  openInspector: inspect => {
    set({ target: null });
    useInspectorStore.getState().focusVersion(inspect.project, inspect.asset, inspect.version);
    useAppStore.getState().setView('inspector');
  },
}));

/**
 * Opens a library model in the quick view: a function per item, undefined
 * for a model the viewer cannot read (an .obj).
 */
export function useLibraryQuickView() {
  const workspace = useAppStore(s => s.workspace);
  const open = useQuickViewStore(s => s.open);
  return (item: LibraryItem): (() => void) | undefined => {
    const target = libraryQuickView(item, workspace);
    return target ? () => open(target) : undefined;
  };
}
