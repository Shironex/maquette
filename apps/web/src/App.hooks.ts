import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';

const NOTICE_MS = 5000;
/** A notice that asks something stays a while longer. */
const ACTION_MS = 12_000;

/** Boots the app, follows the URL hash and checks the tools once the app shows. */
export function useApp() {
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

  return phase;
}

/** Why the backend did not start, and a retry. */
export function useBootFailure() {
  const error = useAppStore(s => s.bootError);
  const init = useAppStore(s => s.init);
  return { error, retry: () => void init() };
}

/** The current notice, dismissed after a few seconds (longer with a button on it). */
export function useNoticeHost() {
  const notice = useAppStore(s => s.notice);
  const dismiss = useAppStore(s => s.dismissNotice);
  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(dismiss, notice.action ? ACTION_MS : NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice, dismiss]);
  const act = () => {
    notice?.action?.run();
    dismiss();
  };
  return { notice, dismiss, act };
}
