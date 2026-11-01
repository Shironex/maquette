import { errorMessage } from '@/ipc/client';
import { useAppStore } from './use-app-store';

/**
 * Asks for Blender's window on a file. Blender takes its time to come up and
 * says nothing while it does, so the click is answered here first.
 */
export async function openInBlender(name: string, open: () => Promise<unknown>): Promise<void> {
  const { notify } = useAppStore.getState();
  notify({
    status: 'info',
    title: `Opening ${name} in Blender`,
    detail: 'Its own window comes up when Blender has started.',
  });
  try {
    await open();
  } catch (error) {
    notify({ status: 'failed', title: 'Could not open Blender', detail: errorMessage(error) });
  }
}
