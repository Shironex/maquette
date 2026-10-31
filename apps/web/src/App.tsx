import { Button, EmptyState, Toast, cn } from '@maquette/ui';
import { useFloatingApprovals } from '@/features/agents/ApprovalPrompt/ApprovalPrompt.hooks';
import { AppShell } from '@/features/shell/AppShell';
import { Onboarding } from '@/features/onboarding/Onboarding';
import { WorkspaceInUseDialog } from '@/features/common/WorkspaceInUseDialog';
import { useApp, useBootFailure, useNoticeHost } from './App.hooks';

export function App() {
  const phase = useApp();

  return (
    <div className="h-full">
      {phase === 'booting' ? <div className="h-full bg-app" aria-busy="true" /> : null}
      {phase === 'failed' ? <BootFailure /> : null}
      {phase === 'onboarding' ? <Onboarding /> : null}
      {phase === 'app' ? <AppShell /> : null}
      <NoticeHost />
      <WorkspaceInUseDialog />
    </div>
  );
}

function BootFailure() {
  const { error, retry } = useBootFailure();
  return (
    <div className="grid h-full place-items-center bg-app">
      <EmptyState
        icon="warning-circle"
        title="Maquette could not start"
        body={error ?? 'The app backend did not answer.'}
        action={
          <Button variant="primary" icon="arrow-clockwise" onClick={retry}>
            Try again
          </Button>
        }
      />
    </div>
  );
}

function NoticeHost() {
  const { notice, dismiss, act } = useNoticeHost();
  const approvals = useFloatingApprovals();
  return (
    <div
      aria-live="polite"
      // An approval prompt holds the corner, so a notice sits beside it.
      className={cn(
        'pointer-events-none fixed bottom-11 z-50',
        approvals?.length ? 'right-[436px]' : 'right-5'
      )}
    >
      {notice ? (
        <Toast
          key={notice.id}
          status={notice.status}
          title={notice.title}
          detail={notice.detail}
          action={
            notice.action ? (
              <Button size="sm" onClick={act}>
                {notice.action.label}
              </Button>
            ) : undefined
          }
          onClose={dismiss}
          className="pointer-events-auto"
        />
      ) : null}
    </div>
  );
}
