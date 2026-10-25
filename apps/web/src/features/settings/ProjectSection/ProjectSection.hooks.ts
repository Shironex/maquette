import { useEffect, useState } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { homePath } from '@/lib/format';
import { useAppStore } from '@/state/use-app-store';

/** The open project's name and site repo fields, saved on commit. */
export function useProjectSection() {
  const workspace = useAppStore(s => s.workspace);
  const setProject = useAppStore(s => s.setProject);
  const notify = useAppStore(s => s.notify);
  const home = useAppStore(s => s.boot?.home);
  const project = workspace?.project;
  const shownRepo = project?.siteRepo ? homePath(project.siteRepo, home) : '';
  const [name, setName] = useState(project?.name ?? '');
  const [repo, setRepo] = useState(shownRepo);

  useEffect(() => {
    setName(project?.name ?? '');
    setRepo(shownRepo);
  }, [project?.id, project?.name, shownRepo]);

  const save = async (nextName: string, nextRepo: string) => {
    if (!project) return;
    if (nextName.trim() === project.name && nextRepo.trim() === shownRepo) return;
    try {
      const updated = await commands.projectUpdate(
        project.id,
        nextName.trim(),
        nextRepo.trim() || null
      );
      // Only this project: a switch or an agent's change may have landed meanwhile.
      setProject(updated);
    } catch (error) {
      setName(project.name);
      notify({
        status: 'failed',
        title: 'Could not save the project',
        detail: errorMessage(error),
      });
    }
  };

  const browse = async () => {
    const picked = await commands.dialogPickFolder('Pick the site repo', repo).catch(() => null);
    if (!picked) return;
    setRepo(picked);
    await save(name, picked);
  };

  return {
    project: workspace ? project : undefined,
    name,
    setName,
    repo,
    setRepo,
    save,
    browse,
  };
}
