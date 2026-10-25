import { useEffect, useState } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import type { RootNote } from './WorkspaceSection.types';

/** The cache size on disk, and clearing it. */
export function useWorkspaceCache() {
  const workspace = useAppStore(s => s.workspace);
  const notify = useAppStore(s => s.notify);
  const [cache, setCache] = useState<number | null>(null);
  const [clearing, setClearing] = useState(false);

  useEffect(() => {
    let live = true;
    commands
      .workspaceCacheSize()
      .then(info => live && setCache(info.bytes ?? 0))
      .catch(() => live && setCache(null));
    return () => {
      live = false;
    };
  }, [workspace?.root]);

  const clear = async () => {
    setClearing(true);
    try {
      const info = await commands.workspaceClearCache();
      setCache(info.bytes ?? 0);
      notify({
        status: 'done',
        title: 'Cache cleared',
        detail: 'Unchanged steps will run again on the next build.',
      });
    } catch (error) {
      notify({ status: 'failed', title: 'Could not clear the cache', detail: errorMessage(error) });
    } finally {
      setClearing(false);
    }
  };

  return { cache, clearing, clear };
}

/** The root folder field: typing, browsing, and switching to another workspace. */
export function useWorkspaceRoot() {
  const workspace = useAppStore(s => s.workspace);
  const setWorkspace = useAppStore(s => s.setWorkspace);
  const [root, setRoot] = useState(workspace?.display ?? '');
  const [note, setNote] = useState<RootNote | undefined>();

  const switchTo = async (path: string) => {
    if (!path.trim() || path === workspace?.display || path === workspace?.root) {
      setNote(undefined);
      return;
    }
    try {
      const found = await commands.workspaceInspect(path);
      if (found.state !== 'workspace') {
        setNote({
          tone: 'missing',
          text: 'Pick a folder that holds maquette.json. Moving a workspace arrives later.',
        });
        return;
      }
      const snapshot = await commands.workspaceOpen(found.path);
      setWorkspace(snapshot);
      setRoot(snapshot.display);
      setNote({ tone: 'found', text: 'Switched to this workspace.' });
    } catch (error) {
      setNote({ tone: 'missing', text: errorMessage(error) });
    }
  };

  const browse = async () => {
    const picked = await commands
      .dialogPickFolder('Open a Maquette workspace', root)
      .catch(() => null);
    if (!picked) return;
    setRoot(picked);
    await switchTo(picked);
  };

  return { root, setRoot, note, switchTo, browse };
}
