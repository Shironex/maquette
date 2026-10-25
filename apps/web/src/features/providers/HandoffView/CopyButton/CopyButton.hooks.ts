import { copyText } from '@/features/config/copyText';
import { useAppStore } from '@/state/use-app-store';

/** Copies `text` and says so, or says the clipboard is not there. */
export function useCopy(text: string, what: string) {
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
