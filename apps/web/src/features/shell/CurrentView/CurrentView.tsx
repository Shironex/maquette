import { Suspense, lazy } from 'react';
import { BoardView } from '@/features/board/BoardView';
import { ProviderQueue } from '@/features/providers/ProviderQueue';
import { PlaceholderView } from '@/features/tools/PlaceholderView';
import type { ViewId } from '@/state/views';
import type { CurrentViewProps } from './CurrentView.types';
import { ViewBoundary } from './ViewBoundary';

// The Board opens first; every other view loads when the rail first picks it.
const InboxView = lazy(() =>
  import('@/features/inbox/InboxView').then(m => ({ default: m.InboxView }))
);
const LabView = lazy(() => import('@/features/lab/LabView').then(m => ({ default: m.LabView })));
const LibraryView = lazy(() =>
  import('@/features/library/LibraryView').then(m => ({ default: m.LibraryView }))
);
const AnimationsView = lazy(() =>
  import('@/features/animations/AnimationsView').then(m => ({ default: m.AnimationsView }))
);
const InspectorView = lazy(() =>
  import('@/features/inspector/InspectorView').then(m => ({ default: m.InspectorView }))
);
const MotionView = lazy(() =>
  import('@/features/motion/MotionView').then(m => ({ default: m.MotionView }))
);
const AvatarView = lazy(() =>
  import('@/features/avatar/AvatarView').then(m => ({ default: m.AvatarView }))
);
const PagePreview = lazy(() =>
  import('@/features/page/PagePreview').then(m => ({ default: m.PagePreview }))
);
const PageView = lazy(() =>
  import('@/features/page/PageView').then(m => ({ default: m.PageView }))
);
const PipelineView = lazy(() =>
  import('@/features/pipeline/PipelineView').then(m => ({ default: m.PipelineView }))
);
const SettingsView = lazy(() =>
  import('@/features/settings/SettingsView').then(m => ({ default: m.SettingsView }))
);
const TimelineView = lazy(() =>
  import('@/features/timeline/TimelineView').then(m => ({ default: m.TimelineView }))
);

function viewFor(view: ViewId) {
  switch (view) {
    case 'board':
      return <BoardView queue={onClose => <ProviderQueue onClose={onClose} />} />;
    case 'pipeline':
      return <PipelineView />;
    case 'inbox':
      return <InboxView />;
    case 'lab':
      return <LabView />;
    case 'settings':
      return <SettingsView />;
    case 'inspector':
      return <InspectorView />;
    case 'motion':
      return <MotionView />;
    case 'avatar':
      return <AvatarView />;
    case 'timeline':
      return <TimelineView preview={<PagePreview />} />;
    case 'page':
      return <PageView />;
    case 'library':
      return <LibraryView />;
    case 'animations':
      return <AnimationsView />;
    default:
      return <PlaceholderView view={view} />;
  }
}

/** The view the rail has selected, with a blank busy view while its code loads. */
export default function CurrentView({ view }: CurrentViewProps) {
  return (
    <ViewBoundary view={view}>
      <Suspense fallback={<main className="grid flex-1 place-items-center" aria-busy="true" />}>
        {viewFor(view)}
      </Suspense>
    </ViewBoundary>
  );
}
