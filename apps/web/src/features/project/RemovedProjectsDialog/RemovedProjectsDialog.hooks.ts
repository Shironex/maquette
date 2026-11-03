import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import { useProjectTrashStore } from '@/state/use-project-trash-store';

const count = (n: number, one: string, many: string) => (n === 1 ? `1 ${one}` : `${n} ${many}`);

/** The removed projects, newest first, each with Restore. */
export function useRemovedProjectsDialog() {
  const open = useProjectTrashStore(s => s.listOpen);
  const removed = useProjectTrashStore(s => s.removed);
  const busy = useProjectTrashStore(s => s.busy);
  const restore = useProjectTrashStore(s => s.restore);
  const close = useProjectTrashStore(s => s.closeList);
  if (!open) return null;
  const now = clock.nowMs();
  return {
    rows: removed.map(r => ({
      trash: r.trash,
      name: r.name,
      meta: `${r.target === 'game' ? 'Game' : 'Website'} · ${count(r.assets, 'asset', 'assets')} · removed ${formatAgo(Date.parse(r.removedAt), now)}`,
    })),
    busy,
    restore: (trash: string) => void restore(trash),
    close,
  };
}
