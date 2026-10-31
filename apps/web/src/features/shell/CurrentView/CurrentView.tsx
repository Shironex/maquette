import { Suspense } from 'react';
import { BoardView } from '@/features/board/BoardView';
import { ViewLoader } from '@/features/common/ViewLoader';
import { PlaceholderView } from '@/features/tools/PlaceholderView';
import { lazyAfterPaint } from '@/lib/after-paint';
import { LIBRARY_VIEWS, SETTINGS_VIEW, TOOL_VIEWS, type ViewId } from '@/state/views';
import type { CurrentViewProps } from './CurrentView.types';
import { usePreloadViews } from './CurrentView.hooks';
import { ViewBoundary } from './ViewBoundary';

// The Board opens first; every other view loads when the rail first picks it,
// or sooner when the app has a quiet moment (see `preloadWhenIdle`).
const InboxView = lazyAfterPaint(async () => ({
  default: (await import('@/features/inbox/InboxView')).InboxView,
}));
const LabView = lazyAfterPaint(async () => ({
  default: (await import('@/features/lab/LabView')).LabView,
}));
const LibraryView = lazyAfterPaint(async () => ({
  default: (await import('@/features/library/LibraryView')).LibraryView,
}));
const AnimationsView = lazyAfterPaint(async () => ({
  default: (await import('@/features/animations/AnimationsView')).AnimationsView,
}));
const InspectorView = lazyAfterPaint(async () => ({
  default: (await import('@/features/inspector/InspectorView')).InspectorView,
}));
const MotionView = lazyAfterPaint(async () => ({
  default: (await import('@/features/motion/MotionView')).MotionView,
}));
const AvatarView = lazyAfterPaint(async () => ({
  default: (await import('@/features/avatar/AvatarView')).AvatarView,
}));
const PagePreview = lazyAfterPaint(async () => ({
  default: (await import('@/features/page/PagePreview')).PagePreview,
}));
const PageView = lazyAfterPaint(async () => ({
  default: (await import('@/features/page/PageView')).PageView,
}));
const PipelineView = lazyAfterPaint(async () => ({
  default: (await import('@/features/pipeline/PipelineView')).PipelineView,
}));
const SettingsView = lazyAfterPaint(async () => ({
  default: (await import('@/features/settings/SettingsView')).SettingsView,
}));
const TimelineView = lazyAfterPaint(async () => ({
  default: (await import('@/features/timeline/TimelineView')).TimelineView,
}));

const PRELOADS = [
  InspectorView.preload,
  MotionView.preload,
  LibraryView.preload,
  AnimationsView.preload,
  InboxView.preload,
  PipelineView.preload,
  LabView.preload,
  AvatarView.preload,
  TimelineView.preload,
  PagePreview.preload,
  PageView.preload,
  SettingsView.preload,
  // three.js and the 3D stages, the largest chunks behind the views.
  () => import('@/features/viewer/ModelPreview'),
];

function viewFor(view: ViewId) {
  switch (view) {
    case 'board':
      return <BoardView />;
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

const labelOf = (view: ViewId) =>
  [...TOOL_VIEWS, ...LIBRARY_VIEWS, SETTINGS_VIEW].find(v => v.id === view)?.label ?? 'the view';

/** The view the rail has selected, with the loader while its code loads. */
export default function CurrentView({ view }: CurrentViewProps) {
  usePreloadViews(PRELOADS);
  return (
    <ViewBoundary view={view}>
      <Suspense fallback={<ViewLoader label={`Opening ${labelOf(view)}...`} />}>
        {viewFor(view)}
      </Suspense>
    </ViewBoundary>
  );
}
