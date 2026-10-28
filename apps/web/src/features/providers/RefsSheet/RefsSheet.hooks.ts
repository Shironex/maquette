import { commands, type RefsView, type WarningsItem } from '@/ipc/client';
import { useAsyncValue } from '@/lib/useAsyncValue';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';

/** The sheet the refs store has open, and Send to 3D from it. */
export function useRefsSheet() {
  const sheet = useRefsStore(s => s.sheet);
  const view = useRefsStore(s => s.view);
  const error = useRefsStore(s => s.error);
  const close = useRefsStore(s => s.close);
  const openPanel = useProvidersStore(s => s.openPanel);
  const send = () => {
    if (!sheet) return;
    close();
    void openPanel(sheet.project, sheet.asset);
  };
  return { sheet, view, error, close, send };
}

const check = ({ project, items }: { project: string; items: WarningsItem[] }) =>
  commands.refsWarnings(project, items).then(lists => lists.flat());

/**
 * The weak ref flags for the sheet's tiles: the views checked as one send,
 * each other ref on its own.
 */
export function useTileWarnings(view: RefsView | null) {
  const arg = view
    ? {
        project: view.project.id,
        items: [
          {
            asset: view.asset.id,
            images: view.refs.filter(r => r.view !== 'other').map(r => r.file),
            multiView: false,
          },
          ...view.refs
            .filter(r => r.view === 'other')
            .map(r => ({ asset: view.asset.id, images: [r.file], multiView: false })),
        ],
      }
    : null;
  return useAsyncValue(check, arg) ?? [];
}
