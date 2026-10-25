import { BoardView } from '@/features/board/BoardView';
import { InboxView } from '@/features/inbox/InboxView';
import { LabView } from '@/features/lab/LabView';
import { InspectorView } from '@/features/inspector/InspectorView';
import { PipelineView } from '@/features/pipeline/PipelineView';
import { ProviderQueue } from '@/features/providers/ProviderQueue';
import { SettingsView } from '@/features/settings/SettingsView';
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
    default:
      return <PlaceholderView view={view} />;
  }
}
