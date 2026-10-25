import { Logo, NavRail, TrafficLights, cn } from '@maquette/ui';
import { CHROME } from '@/ipc/runtime';
import { NewProjectDialog } from '@/features/project/NewProjectDialog';
import { ApprovalPrompt } from '@/features/agents/ApprovalPrompt';
import { BatchSendPanel } from '@/features/providers/BatchSendPanel';
import { CreditsSheet } from '@/features/providers/CreditsSheet';
import { DownloadsMatches } from '@/features/providers/DownloadsMatches';
import { HandoffChecklist } from '@/features/providers/HandoffChecklist';
import { SendPanel } from '@/features/providers/SendPanel';
import { ObjectRefsDialog } from '@/features/lab/ObjectRefsDialog';
import { SETTINGS_VIEW, type ViewId } from '@/state/views';
import { CurrentView } from '../CurrentView';
import { ProjectSwitcherMenu } from '../ProjectSwitcherMenu';
import { QuitDialog } from '../QuitDialog';
import { ShellStatusBar } from '../ShellStatusBar';
import { useAppShell } from './AppShell.hooks';

export default function AppShell() {
  const {
    view,
    setView,
    items,
    collapsed,
    toggleRail,
    switcher,
    switcherButton,
    shortcut,
    focusSwitcher,
  } = useAppShell();

  return (
    <div className="flex h-full min-h-0 bg-app text-fg">
      <NavRail<ViewId>
        items={items}
        footer={SETTINGS_VIEW}
        value={view}
        onChange={setView}
        collapsed={collapsed}
        onToggleCollapsed={toggleRail}
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
              <Logo size={24} wordmark={!collapsed} />
            </div>
          </div>
        }
        project={switcherButton}
        projectMenu={
          <ProjectSwitcherMenu anchorRef={switcher} collapsed={collapsed} shortcut={shortcut} />
        }
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex min-h-0 flex-1 flex-col">
          <CurrentView view={view} />
        </div>
        <ShellStatusBar />
      </div>
      <HandoffChecklist />
      <BatchSendPanel />
      <SendPanel />
      <CreditsSheet />
      <ObjectRefsDialog />
      <DownloadsMatches />
      <NewProjectDialog onClosed={focusSwitcher} />
      <ApprovalPrompt />
      <QuitDialog />
    </div>
  );
}
