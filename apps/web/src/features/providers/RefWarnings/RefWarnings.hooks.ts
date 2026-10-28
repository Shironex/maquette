import { commands, type RefFix, type RefWarning, type WarningsItem } from '@/ipc/client';
import { useAsyncValue } from '@/lib/useAsyncValue';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';

const check = ({ project, items }: { project: string; items: WarningsItem[] }) =>
  commands.refsWarnings(project, items);

/** The weak ref warnings of each item, asked again as the picks change; null until known. */
export function useRefWarnings(project: string | null, items: WarningsItem[]) {
  return useAsyncValue(check, project && items.length > 0 ? { project, items } : null);
}

/**
 * The one click fixes: Leave out drops the image from the send through
 * `leaveOut`; the others close the send panels and open the asset's refs
 * sheet at Make the other views or Clean background.
 */
export function useWarningFix(project: string, leaveOut: (asset: string, file: string) => void) {
  const openRefs = useRefsStore(s => s.open);
  const closePanel = useProvidersStore(s => s.closePanel);
  const closeBatch = useProvidersStore(s => s.closeBatch);
  return (asset: string, warning: RefWarning, fix: RefFix) => {
    if (fix === 'leave-out') {
      if (warning.file) leaveOut(asset, warning.file);
      return;
    }
    closePanel();
    closeBatch();
    void openRefs(
      project,
      asset,
      fix === 'make-views' ? 'views' : 'clean',
      fix === 'clean-background' ? warning.file : null
    );
  };
}
