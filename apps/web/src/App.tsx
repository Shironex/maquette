import { useEffect } from 'react';
import { Button, EmptyState, Toast } from '@maquette/ui';
import { AppShell } from '@/features/shell/AppShell';
import { Onboarding } from '@/features/onboarding/Onboarding';
import { useAppStore } from '@/state/app-store';

export function App() {
  const phase = useAppStore(s => s.phase);
  const init = useAppStore(s => s.init);
  const syncView = useAppStore(s => s.syncViewFromHash);
  const detectTools = useAppStore(s => s.detectTools);
  const hasReports = useAppStore(s => s.tools.reports.length > 0);

  useEffect(() => {
    void init();
  }, [init]);

  useEffect(() => {
    window.addEventListener('hashchange', syncView);
    return () => window.removeEventListener('hashchange', syncView);
  }, [syncView]);

  // The shell shows Blender in the status bar and Settings lists every tool.
  useEffect(() => {
    if (phase === 'app' && !hasReports) void detectTools();
  }, [phase, hasReports, detectTools]);

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
  const error = useAppStore(s => s.bootError);
  const init = useAppStore(s => s.init);
  return (
    <div className="grid h-full place-items-center bg-app">
      <EmptyState
        icon="warning-circle"
        title="Maquette could not start"
        body={error ?? 'The app backend did not answer.'}
        action={
          <Button variant="primary" icon="arrow-clockwise" onClick={() => void init()}>
            Try again
          </Button>
        }
      />
    </div>
  );
}

const NOTICE_MS = 5000;

function NoticeHost() {
  const notice = useAppStore(s => s.notice);
  const dismiss = useAppStore(s => s.dismissNotice);
  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(dismiss, NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice, dismiss]);
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
