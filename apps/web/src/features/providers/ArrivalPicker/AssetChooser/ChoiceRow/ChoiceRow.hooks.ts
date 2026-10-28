import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';

/** The URL of an asset's picture, relative to the open project's folder. */
export function useChoicePicture(picture: string | null) {
  const workspace = useAppStore(s => s.workspace);
  return fileUrl(picture && workspace ? projectFile(workspace, picture) : null);
}
