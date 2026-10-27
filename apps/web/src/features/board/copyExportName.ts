import { copyText } from '@/features/config/copyText';
import { useAppStore } from '@/state/use-app-store';

/** Copies a By hand send's export name and says so, with the name. */
export async function copyExportName(name: string): Promise<void> {
  const ok = await copyText(name);
  useAppStore.getState().notify(
    ok
      ? { status: 'done', title: 'Export name copied', detail: name }
      : {
          status: 'failed',
          title: 'Could not copy',
          detail: 'The clipboard is not available here.',
        }
  );
}
