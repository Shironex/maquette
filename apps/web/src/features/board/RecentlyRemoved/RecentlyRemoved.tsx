import { Menu, type MenuItem } from '@/features/common/Menu';
import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import type { TrashEntry } from '@/ipc/client';

export interface RecentlyRemovedProps {
  entries: readonly TrashEntry[];
  restoring: string | null;
  restoreOne: (id: string) => void;
  onClose: () => void;
}

/** Recently removed: the assets in the project's trash, restoring one with a click. */
export default function RecentlyRemoved({
  entries,
  restoring,
  restoreOne,
  onClose,
}: RecentlyRemovedProps) {
  const now = clock.nowMs();
  const items: MenuItem[] = entries.map(e => ({
    id: e.id,
    label: e.name,
    icon: 'arrow-counter-clockwise',
    disabled: restoring !== null,
    hint:
      restoring === e.id
        ? 'Restoring'
        : `Removed ${formatAgo(Date.parse(e.removedAt), now)}. Restore puts it and its ${e.files === 1 ? 'file' : `${e.files} files`} back.`,
    onSelect: () => restoreOne(e.id),
  }));

  return (
    <Menu
      label="Recently removed"
      onClose={onClose}
      className="absolute top-full right-0 mt-1 w-[260px]"
      header={
        <p className="px-2.5 pt-1.5 pb-1 text-meta text-fg-mute">
          <span className="mb-0.5 block text-caption tracking-[0.06em] uppercase">
            Recently removed
          </span>
          Pick one to put it back on the Board with its files.
        </p>
      }
      items={items}
    />
  );
}
