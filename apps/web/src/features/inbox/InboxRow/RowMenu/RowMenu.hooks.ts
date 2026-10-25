import type { MenuItem } from '@/features/common/Menu';
import type { InboxItem } from '@/ipc/client';
import { batchRedoPlan, redoBlocker } from '@/state/redo';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** A row's menu: Redo it, redo the ticked models together, tick it, reprocess or open it. */
export function useRowMenu(item: InboxItem): MenuItem[] {
  const openRedo = useProvidersStore(s => s.openRedo);
  const openBatchRedo = useProvidersStore(s => s.openBatchRedo);
  const { snapshot, checked, check, clearChecked, reprocess, openInBlender } = useInboxStore();
  const blocker = redoBlocker(item);
  const isChecked = checked.includes(item.id);
  const plan = batchRedoPlan(snapshot?.items ?? [], checked);
  const link = item.link;
  return [
    {
      id: 'redo',
      label: 'Redo...',
      icon: 'arrow-bend-up-right',
      disabled: Boolean(blocker),
      hint: blocker ?? 'Send it again with the same settings, changed where you like',
      onSelect: () => link && void openRedo(link.project, link.asset, link.job),
    },
    ...(checked.length > 1
      ? [
          {
            id: 'redo-checked',
            label: `Redo ${checked.length} selected...`,
            icon: 'stack' as const,
            disabled: Boolean(plan.blocker),
            hint: plan.blocker ?? 'One batch, each model prefilled from its own send',
            onSelect: () => {
              if (!plan.project) return;
              void openBatchRedo(plan.project, plan.redo);
              clearChecked();
            },
          },
        ]
      : []),
    {
      id: 'check',
      label: isChecked ? 'Deselect' : 'Select',
      icon: 'check',
      onSelect: () => check(item.id, !isChecked),
    },
    {
      id: 'reprocess',
      label: 'Reprocess',
      icon: 'arrow-clockwise',
      onSelect: () => void reprocess(item.id),
    },
    {
      id: 'blender',
      label: 'Open in Blender',
      icon: 'arrow-square-out',
      onSelect: () => void openInBlender(item.id),
    },
  ];
}
