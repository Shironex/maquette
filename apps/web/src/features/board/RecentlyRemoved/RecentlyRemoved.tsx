import { Button } from '@maquette/ui';
import { Menu, type MenuItem } from '@/features/common/Menu';
import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import { useRecentlyRemoved } from './RecentlyRemoved.hooks';

/**
 * Recently removed: a button beside the Board's others while the project's
 * trash holds assets, and the list that restores one with a click.
 */
export default function RecentlyRemoved() {
  const { entries, restoring, open, setOpen, restoreOne } = useRecentlyRemoved();
  if (entries.length === 0) return null;
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
    <div className="relative">
      <Button
        icon="trash"
        aria-expanded={open}
        tooltip="Recently removed: assets in the workspace trash, ready to restore"
        onClick={() => setOpen(!open)}
      >
        Removed
        <span className="ml-0.5 rounded-pill bg-raised px-1.5 font-mono text-[11px] leading-[18px] text-fg-dim">
          {entries.length}
        </span>
      </Button>
      {open ? (
        <Menu
          label="Recently removed"
          onClose={() => setOpen(false)}
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
      ) : null}
    </div>
  );
}
