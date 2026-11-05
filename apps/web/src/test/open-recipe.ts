/**
 * Opens the mock workspace with the Motion demo models and the library's
 * demo recipes (`?motion=demo&library=demo` in the browser), loads
 * Frostveil's motion and opens one recipe in the editor.
 */
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { openMockWorkspace } from './mock-ipc';

export async function openRecipe(id = 'reach-for-the-lantern') {
  useMotionStore.setState(useMotionStore.getInitialState(), true);
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
  useRecipeEditorStore.setState(useRecipeEditorStore.getInitialState(), true);
  const args = Array<string | undefined>(11).fill(undefined);
  args[0] = 'configured';
  args[6] = 'demo';
  args[11] = 'demo';
  await openMockWorkspace(...(args as Parameters<typeof openMockWorkspace>));
  await useMotionStore.getState().load('frostveil');
  await useRecipeEditorStore.getState().open('frostveil', id);
  return useRecipeEditorStore.getState();
}

export { clearMocks } from './mock-ipc';
