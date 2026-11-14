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

export interface KeptShare {
  /** "24%" of the source, "+157%" when it grew, "·" unmeasured. */
  text: string;
  /** True when the shipped number is larger than the source. */
  grew: boolean;
}

/** The share of the source a shipped number keeps, or how much it grew. */
export function keptShare(source: number | null, shipped: number | null): KeptShare {
  if (!source || shipped === null) return { text: '·', grew: false };
  if (shipped > source) {
    return { text: `+${Math.round((shipped / source - 1) * 100)}%`, grew: true };
  }
  return { text: `${Math.round((shipped / source) * 100)}%`, grew: false };
}
