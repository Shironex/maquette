import { useEffect, useState } from 'react';
import { commands, errorMessage, type PageSettings } from '@/ipc/client';
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

/**
 * The project's own command for the site's dev server, saved with the
 * page settings. Empty means Maquette reads it from the site, which the
 * placeholder shows.
 */
export function useDevCommand(projectId: string | null, siteRepo: string | null) {
  const notify = useAppStore(s => s.notify);
  const [settings, setSettings] = useState<PageSettings | null>(null);
  const [detected, setDetected] = useState<string | null>(null);
  const [value, setValue] = useState('');

  useEffect(() => {
    if (!projectId) return;
    let live = true;
    commands
      .pageGet(projectId)
      .then(view => {
        if (!live) return;
        setSettings(view.settings);
        setValue(view.settings.devCommand ?? '');
        setDetected(
          view.command && view.command.source !== 'settings' ? view.command.argv.join(' ') : null
        );
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [projectId, siteRepo]);

  const save = async () => {
    if (!projectId || !settings) return;
    const next = value.trim() || null;
    if (next === settings.devCommand) return;
    try {
      const view = await commands.pageSave(projectId, { ...settings, devCommand: next });
      setSettings(view.settings);
      setValue(view.settings.devCommand ?? '');
    } catch (error) {
      setValue(settings.devCommand ?? '');
      notify({
        status: 'failed',
        title: 'Could not save the dev command',
        detail: errorMessage(error),
      });
    }
  };

  return {
    ready: settings !== null,
    value,
    setValue,
    save,
    placeholder: detected ?? (siteRepo ? 'Read from the site' : 'Pick the site repo first'),
  };
}
