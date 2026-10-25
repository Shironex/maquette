import { Menu } from '@/features/common/Menu';
import type { InboxItem } from '@/ipc/client';
import { itemTitle } from '@/state/inbox';
import { useRowMenu } from './RowMenu.hooks';

export interface RowMenuProps {
  item: InboxItem;
  onClose: () => void;
}

/** The actions of one inbox model, Redo first. */
export default function RowMenu({ item, onClose }: RowMenuProps) {
  const items = useRowMenu(item);
  return <Menu label={`${itemTitle(item)} actions`} items={items} onClose={onClose} />;
}
