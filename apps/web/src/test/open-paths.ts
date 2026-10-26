/**
 * Opens the mock workspace with the Paths demo (`?paths=` in the browser)
 * and loads Moon Island into the Paths store.
 */
import { usePathsStore } from '@/state/use-paths-store';
import { openMockWorkspace } from './mock-ipc';

export async function openPaths(paths = 'demo') {
  usePathsStore.setState(usePathsStore.getInitialState(), true);
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    paths
  );
  await usePathsStore.getState().load('moon-island');
  return usePathsStore.getState();
}

export { clearMocks } from './mock-ipc';
