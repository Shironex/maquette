/**
 * Opens the mock workspace with the Timeline demo (`?timeline=` in the
 * browser) and loads Frostveil into the Timeline store.
 */
import { useTimelineStore } from '@/state/use-timeline-store';
import { openMockWorkspace } from './mock-ipc';

/** `page` picks the mock page server's state, as `?page=` does. */
export async function openTimeline(timeline = 'demo', page?: string) {
  useTimelineStore.setState(useTimelineStore.getInitialState(), true);
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    page,
    timeline
  );
  await useTimelineStore.getState().load('frostveil');
  return useTimelineStore.getState();
}

export { clearMocks } from './mock-ipc';
