import { firstToReview } from '@/state/inbox';
import { useInboxStore } from '@/state/use-inbox-store';

/** The inbox listing and the item on show: the picked one, else the first to review. */
export function useInboxView() {
  const { snapshot, selected, select } = useInboxStore();
  const items = snapshot?.items ?? [];
  const item = items.find(i => i.id === selected) ?? firstToReview(items);
  return { snapshot, items, item, select };
}
