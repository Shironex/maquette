/**
 * The folder pickers and the site or game repo check behind the project form, for
 * onboarding and for New project in the switcher.
 */
import { useCallback } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
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

  /** Checks the repo the way the target needs: a site, or a game and its models. */
  const checkRepo = async (path: string, target = fields.target) => {
    // Leaving the field after a check (to click Create) keeps what it found.
    const found = target === 'game' ? fields.game : fields.repo;
    if (found && path === fields.siteRepo) return;
    update({ siteRepo: path, repo: null, game: null });
    if (!path.trim()) return;
    try {
      if (target === 'game') {
        const game = await commands.gameInspectRepo(path);
        update({
          game,
          gameFormat: game.format ?? fields.gameFormat,
          modelsDir: game.modelsDir ?? fields.modelsDir,
        });
      } else {
        update({ repo: await commands.projectInspectRepo(path) });
      }
    } catch (error) {
      notify({
        status: 'failed',
        title: 'Could not check that folder',
        detail: errorMessage(error),
      });
    }
  };

  const browseRepo = async () => {
    const title = fields.target === 'game' ? 'Pick the game repo' : 'Pick the site repo';
    const picked = await pickFolder(title, fields.siteRepo);
    if (picked) await checkRepo(picked);
  };

  const browseSource = async () => {
    const picked = await pickFolder('Pick the folder to import', fields.sourceFolder);
    if (picked) update({ sourceFolder: picked });
  };

  return { checkRepo, browseRepo, browseSource };
}

export type ProjectFieldActions = ReturnType<typeof useProjectFieldActions>;
