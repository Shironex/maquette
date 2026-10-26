/**
 * Opens the mock workspace with the Motion demo models (`?motion=` in the
 * browser) and loads Moon Island, or Rooftop cinema for `still`, into the
 * Motion store.
 */
import { useMotionStore } from '@/state/use-motion-store';
import { openMockGame, openMockWorkspace } from './mock-ipc';

export async function openMotion(motion = 'demo', asset?: string) {
  useMotionStore.setState(useMotionStore.getInitialState(), true);
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    motion
  );
  await useMotionStore.getState().load(motion === 'still' ? 'rooftop-cinema' : 'moon-island');
  if (asset) useMotionStore.getState().select(asset);
  return useMotionStore.getState();
}

/** The Crypt game with its rigged heroine (187 clips) loaded into the Motion store. */
export async function openGameMotion(asset = 'heroine') {
  useMotionStore.setState(useMotionStore.getInitialState(), true);
  await openMockGame('hero');
  await useMotionStore.getState().load('crypt-game');
  useMotionStore.getState().select(asset);
  return useMotionStore.getState();
}

export { clearMocks } from './mock-ipc';
