import { useEffect, useState } from 'react';
import type { ProjectManifest } from '@/ipc/client';
import { gameOf } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useGameStore } from '@/state/use-game-store';

/** The Add split button: the add form, and its menu's other ways in. */
export function useAddMenu() {
  const project = useAppStore(s => s.workspace?.project);
  const setProject = useAppStore(s => s.setProject);
  const setAddingBuilt = useBuiltStore(s => s.setAdding);
  const openImport = useGameStore(s => s.openImport);
  const [adding, setAdding] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  // The add form belongs to the project it was opened in.
  useEffect(() => {
    setAdding(false);
  }, [project?.id]);

  return {
    project,
    game: gameOf(project),
    adding,
    setAdding,
    menuOpen,
    setMenuOpen,
    added: (next: ProjectManifest) => {
      setProject(next);
      setAdding(false);
    },
    addBuilt: () => {
      setAdding(false);
      setAddingBuilt(true);
    },
    importModels: () => {
      if (project) void openImport(project.id);
    },
  };
}
