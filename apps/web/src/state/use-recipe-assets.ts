/**
 * The assets the recipe editor picks props from and names fills with: the
 * open recipe's project, not whichever project is open now. After a project
 * switch the person chose to stay on, they are the ones kept when the
 * recipe opened.
 */
import type { Asset } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useRecipeEditorStore } from './use-recipe-editor-store';

const NONE: readonly Asset[] = [];

/** The open recipe's project's assets: live while that project is open, else as they were. */
export function useRecipeAssets(): readonly Asset[] {
  const project = useRecipeEditorStore(s => s.editing?.project ?? null);
  const kept = useRecipeEditorStore(s => s.editing?.assets ?? NONE);
  const open = useAppStore(s => s.workspace?.project ?? null);
  return open && open.id === project ? (open.assets ?? NONE) : kept;
}
