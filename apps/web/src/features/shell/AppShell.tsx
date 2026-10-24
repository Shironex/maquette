import { Logo, NavRail, StatusBar, TrafficLights, cn, type StatusBarItem } from '@maquette/ui';
import { CHROME } from '@/ipc/runtime';
import { useAppStore } from '@/state/app-store';
import { BoardView } from '@/features/board/BoardView';
import { SettingsView } from '@/features/settings/SettingsView';
import { PlaceholderView } from '@/features/tools/PlaceholderView';
import { projectMeta, projectThumb } from './projectArt';
import { SETTINGS_VIEW, TOOL_VIEWS, type ViewId } from './views';

export function AppShell() {
  const view = useAppStore(s => s.view);
  const setView = useAppStore(s => s.setView);
  const workspace = useAppStore(s => s.workspace);
  const project = workspace?.project ?? null;

  return (
    <div className="flex h-full min-h-0 bg-app text-fg">
      <NavRail<ViewId>
        items={TOOL_VIEWS}
        footer={SETTINGS_VIEW}
        value={view}
        onChange={setView}
        header={
          <div data-tauri-drag-region>
            <TrafficLights mode={CHROME} />
            <div
              data-tauri-drag-region
              className={cn(
                'flex items-center px-1.5 pb-3.5',
                CHROME === 'native' ? 'pt-1' : 'pt-[18px]'
              )}
            >
              <Logo size={24} />
            </div>
          </div>
        }
        project={
          project
            ? {
                name: project.name,
                meta: projectMeta(project),
                thumb: projectThumb(project),
                onClick: () => setView('settings'),
              }
            : undefined
        }
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex min-h-0 flex-1 flex-col">
          <CurrentView view={view} />
        </div>
        <ShellStatusBar />
      </div>
    </div>
  );
}

function CurrentView({ view }: { view: ViewId }) {
  switch (view) {
    case 'board':
      return <BoardView />;
    case 'settings':
      return <SettingsView />;
    default:
      return <PlaceholderView view={view} />;
  }
}

function ShellStatusBar() {
  const workspace = useAppStore(s => s.workspace);
  const { reports, checking } = useAppStore(s => s.tools);
  const blender = reports.find(r => r.id === 'blender');
  const blenderItem: StatusBarItem = checking
    ? { label: 'Checking tools', dot: 'running' }
    : blender?.status === 'found'
      ? { label: `Blender ${blender.version ?? ''}, headless`.replace('  ', ' '), icon: 'cube' }
      : { label: 'Blender not found', dot: 'offline' };
  const inbox = workspace ? `${workspace.display}/${workspace.config.layout.inbox}` : null;
  const items: StatusBarItem[] = [
    { label: 'Local worker connected', dot: 'ok' },
    blenderItem,
    ...(inbox ? [{ label: `Inbox at ${inbox}`, icon: 'tray' as const }] : []),
    { label: 'No runs yet', right: true },
  ];
  return <StatusBar items={items} />;
}
