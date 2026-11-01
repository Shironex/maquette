import { Suspense, lazy } from 'react';
import { Logo, NavRail, TrafficLights, cn } from '@maquette/ui';
import { CHROME } from '@/ipc/runtime';
import { NewProjectDialog } from '@/features/project/NewProjectDialog';
import { GameImportDialog } from '@/features/game/GameImportDialog';
import { ApprovalPrompt } from '@/features/agents/ApprovalPrompt';
import { ArrivalPicker } from '@/features/providers/ArrivalPicker';
import { BatchSendPanel } from '@/features/providers/BatchSendPanel';
import { CreditsSheet } from '@/features/providers/CreditsSheet';
import { DownloadsMatches } from '@/features/providers/DownloadsMatches';
import { SendsPanel } from '@/features/providers/SendsPanel';
import { SendPanel } from '@/features/providers/SendPanel';
import { RefsSheet } from '@/features/providers/RefsSheet';
import { AssembleSheet } from '@/features/providers/AssembleSheet';
import { SplitSheet } from '@/features/providers/SplitSheet';
import { ObjectRefsDialog } from '@/features/lab/ObjectRefsDialog';
import { StyleBoardDialog } from '@/features/lab/StyleBoardDialog';
import { LibraryDialogs } from '@/features/common/LibraryDialogs';
import { QuickView } from '@/features/viewer/QuickView';
import { AnimationDialogs } from '@/features/animations/AnimationDialogs';
import { RetargetDialog } from '@/features/retarget/RetargetDialog';
import { SETTINGS_VIEW, showsView, type ViewId } from '@/state/views';
import { CurrentView } from '../CurrentView';
import { ProjectSwitcherMenu } from '../ProjectSwitcherMenu';
import { QuitDialog } from '../QuitDialog';
import { RepoPrompt } from '../RepoPrompt';
import { RenderSheet } from '@/features/renders/RenderSheet';
import { ShellStatusBar } from '../ShellStatusBar';
import { SyncConflictBanner } from '../SyncConflictBanner';
import { UpdatePill } from '../UpdatePill';
import { useAppShell } from './AppShell.hooks';

// About 8 kB gzipped that most sessions never open, so it stays out of the first chunk.
const PackSheet = lazy(async () => ({
  default: (await import('@/features/library/PackSheet')).PackSheet,
}));

export default function AppShell() {
  const {
    packOpen,
    view,
    goTo,
    items,
    groups,
    planned,
    target,
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
        groups={groups}
        planned={planned}
        footer={SETTINGS_VIEW}
        value={view}
        onChange={goTo}
        collapsed={collapsed}
        onToggleCollapsed={toggleRail}
        header={
          <div data-tauri-drag-region>
            <TrafficLights mode={CHROME} />
            <div
              data-tauri-drag-region
              className={cn(
                'flex items-center gap-2 px-1.5 pb-3.5',
                collapsed ? 'flex-col' : 'flex-wrap gap-y-2',
                CHROME === 'native' ? 'pt-1' : 'pt-[18px]'
              )}
            >
              <Logo size={24} wordmark={!collapsed} />
              <UpdatePill collapsed={collapsed} />
            </div>
          </div>
        }
        project={switcherButton}
        projectMenu={
          <ProjectSwitcherMenu anchorRef={switcher} collapsed={collapsed} shortcut={shortcut} />
        }
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <SyncConflictBanner />
        <div className="flex min-h-0 flex-1 flex-col">
          <CurrentView view={showsView(target, view) ? view : 'board'} />
        </div>
        <ShellStatusBar />
      </div>
      <SendsPanel />
      <BatchSendPanel />
      <SendPanel />
      <RefsSheet />
      <CreditsSheet />
      <SplitSheet />
      <RenderSheet />
      {packOpen ? (
        <Suspense fallback={null}>
          <PackSheet />
        </Suspense>
      ) : null}
      <AssembleSheet />
      <ObjectRefsDialog />
      <StyleBoardDialog />
      <DownloadsMatches />
      <ArrivalPicker />
      <NewProjectDialog onClosed={focusSwitcher} />
      <GameImportDialog />
      <LibraryDialogs />
      <QuickView />
      <AnimationDialogs />
      <RetargetDialog />
      <ApprovalPrompt />
      <RepoPrompt />
      <QuitDialog />
    </div>
  );
}
