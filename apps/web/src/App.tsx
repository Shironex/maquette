import { Button, EmptyState, Toast } from '@maquette/ui';
import { AppShell } from '@/features/shell/AppShell';
import { Onboarding } from '@/features/onboarding/Onboarding';
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
  const { notice, dismiss } = useNoticeHost();
  return (
    <div aria-live="polite" className="pointer-events-none fixed right-5 bottom-11 z-50">
      {notice ? (
        <Toast
          key={notice.id}
          status={notice.status}
          title={notice.title}
          detail={notice.detail}
          onClose={dismiss}
          className="pointer-events-auto"
        />
      ) : null}
    </div>
  );
}
