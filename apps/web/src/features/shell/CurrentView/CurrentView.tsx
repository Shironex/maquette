import { BoardView } from '@/features/board/BoardView';
import { InboxView } from '@/features/inbox/InboxView';
import { LabView } from '@/features/lab/LabView';
import { LibraryView } from '@/features/library/LibraryView';
import { AnimationsView } from '@/features/animations/AnimationsView';
import { InspectorView } from '@/features/inspector/InspectorView';
import { MotionView } from '@/features/motion/MotionView';
import { PagePreview } from '@/features/page/PagePreview';
import { PageView } from '@/features/page/PageView';
import { PipelineView } from '@/features/pipeline/PipelineView';
import { ProviderQueue } from '@/features/providers/ProviderQueue';
import { SettingsView } from '@/features/settings/SettingsView';
import { TimelineView } from '@/features/timeline/TimelineView';
import { PlaceholderView } from '@/features/tools/PlaceholderView';
import type { CurrentViewProps } from './CurrentView.types';

/** The view the rail has selected. */
export default function CurrentView({ view }: CurrentViewProps) {
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
