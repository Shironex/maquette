import { fileUrl } from '@/ipc/files';
import { characters } from '@/state/avatar';
import { projectFile } from '@/state/board';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';

/** The project's characters with a model, each with its thumbnail. */
export function useAvatarList() {
  const workspace = useAppStore(s => s.workspace);
  const asset = useAvatarStore(s => s.asset);
  const select = useAvatarStore(s => s.select);
  const rows = characters(workspace?.project).map(a => ({
    id: a.id,
    name: a.name,
    line: `v${a.versions.at(-1)?.version ?? 1}`,
    thumb: a.thumbnail && workspace ? fileUrl(projectFile(workspace, a.thumbnail)) : null,
  }));
  return { rows, asset, select };
}
