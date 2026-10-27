import { fileManagerName, revealLabel } from './file-manager';
import { useAppStore } from './use-app-store';

/** What this OS calls its file manager, and the words for showing a file in it. */
export function useFileManager() {
  const os = useAppStore(s => s.boot?.os);
  return { name: fileManagerName(os), reveal: (what: string) => revealLabel(os, what) };
}
