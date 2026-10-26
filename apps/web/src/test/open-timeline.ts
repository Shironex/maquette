/**
 * Opens the mock workspace with the Timeline demo (`?timeline=` in the
 * browser) and loads Moon Island into the Timeline store.
 */
import { useTimelineStore } from '@/state/use-timeline-store';
import { openMockWorkspace } from './mock-ipc';

export async function openTimeline(timeline = 'demo') {
  useTimelineStore.setState(useTimelineStore.getInitialState(), true);
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    timeline
  );
  await useTimelineStore.getState().load('moon-island');
  return useTimelineStore.getState();
}

export { clearMocks } from './mock-ipc';
