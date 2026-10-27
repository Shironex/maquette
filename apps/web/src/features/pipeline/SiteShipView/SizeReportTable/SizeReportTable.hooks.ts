import { copyText } from '@/features/config/copyText';
import { useAppStore } from '@/state/use-app-store';

/** Copies `text` and says so, or says the clipboard is not there. */
export function useCopyReport(text: string, what: string) {
  const notify = useAppStore(s => s.notify);
  return () =>
    void copyText(text).then(ok =>
      notify(
        ok
          ? { status: 'done', title: `${what} copied` }
          : {
              status: 'failed',
              title: 'Could not copy',
              detail: 'The clipboard is not available here.',
            }
      )
    );
}

/** The share of the source a shipped number keeps, "24%". */
export function keptShare(source: number | null, shipped: number | null): string {
  if (!source || shipped === null) return '·';
  return `${Math.round((shipped / source) * 100)}%`;
}
