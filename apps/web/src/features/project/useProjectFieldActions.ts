/**
 * The folder pickers and the site repo check behind the project form, for
 * onboarding and for New project in the switcher.
 */
import { useCallback } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { useAppStore } from '@/state/app-store';
import type { ProjectFields } from '@/state/onboarding';

export function useFolderPicker() {
  const notify = useAppStore(s => s.notify);
  return useCallback(
    async (title: string, start?: string) => {
      try {
        return await commands.dialogPickFolder(title, start?.trim() ? start : null);
      } catch (error) {
        notify({
          status: 'failed',
          title: 'Could not open the folder picker',
          detail: errorMessage(error),
        });
        return null;
      }
    },
    [notify]
  );
}

export function useProjectFieldActions(
  fields: ProjectFields,
  update: (patch: Partial<ProjectFields>) => void
) {
  const notify = useAppStore(s => s.notify);
  const pickFolder = useFolderPicker();

  const checkRepo = async (path: string) => {
    update({ siteRepo: path, repo: null });
    if (!path.trim()) return;
    try {
      update({ repo: await commands.projectInspectRepo(path) });
    } catch (error) {
      notify({
        status: 'failed',
        title: 'Could not check that folder',
        detail: errorMessage(error),
      });
    }
  };

  const browseRepo = async () => {
    const picked = await pickFolder('Pick the site repo', fields.siteRepo);
    if (picked) await checkRepo(picked);
  };

  const browseSource = async () => {
    const picked = await pickFolder('Pick the folder to import', fields.sourceFolder);
    if (picked) update({ sourceFolder: picked });
  };

  return { checkRepo, browseRepo, browseSource };
}

export type ProjectFieldActions = ReturnType<typeof useProjectFieldActions>;
