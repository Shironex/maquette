import { useState } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { readFlag, writeFlag } from '@/lib/storage';
import { needsRepo } from '@/state/sync';
import { useAppStore } from '@/state/use-app-store';

/** Remembers "Not now" per workspace and project, on this computer only. */
const laterKey = (root: string, id: string) => `maquette.repo.later.${root}::${id}`;

/** The open project when it needs asking, and Choose folder and Not now. */
export function useRepoPrompt() {
  const root = useAppStore(s => s.workspace?.root ?? '');
  const project = useAppStore(s => s.workspace?.project ?? null);
  const setProject = useAppStore(s => s.setProject);
  const [later, setLater] = useState<string[]>([]);
  const [picking, setPicking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const key = project ? laterKey(root, project.id) : '';
  const asked = project !== null && needsRepo(project) && !later.includes(key) && !readFlag(key);

  const notNow = () => {
    writeFlag(key, true);
    setLater(keys => [...keys, key]);
    setError(null);
  };

  const choose = async () => {
    if (!project) return;
    setPicking(true);
    setError(null);
    try {
      const title = project.target === 'game' ? 'Pick the game repo' : 'Pick the site repo';
      const picked = await commands.dialogPickFolder(title, null);
      if (picked) setProject(await commands.projectUpdate(project.id, project.name, picked));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPicking(false);
    }
  };

  return { project: asked ? project : null, picking, error, choose, notNow };
}
