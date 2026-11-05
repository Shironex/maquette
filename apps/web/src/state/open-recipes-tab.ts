/**
 * Opens a project's Recipes tab from elsewhere (an agent's approval card,
 * a recipe used from the Animations library): the project, the Motion
 * view and its Recipes tab. False when the project could not be opened.
 */
import { useAppStore } from './use-app-store';
import { useMotionStore } from './use-motion-store';
import { useProjectsStore } from './use-projects-store';

export async function openRecipesTab(project: string): Promise<boolean> {
  if (useAppStore.getState().workspace?.project?.id !== project) {
    const switched = await useProjectsStore.getState().switchTo(project);
    if (!switched) return false;
  }
  useMotionStore.getState().setTab('recipes');
  useAppStore.getState().setView('motion');
  return true;
}
