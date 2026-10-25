import type { Check, InboxItem } from '@/ipc/client';
import { EMPTY_PREP, describeFixup, hasFixup, withFixup, withoutFixup } from '@/state/prepare';
import { usePrepareStore } from '@/state/use-prepare-store';

/** Each fix a check offers, whether the item's draft has it and the toggle. */
export function useCheckFixes(item: InboxItem, check: Check) {
  const draft = usePrepareStore(s => s.drafts[item.id]) ?? EMPTY_PREP;
  const setDraft = usePrepareStore(s => s.setDraft);
  return check.fixes.map(fix => {
    const added = hasFixup(draft, fix);
    return {
      fix,
      added,
      text: describeFixup(fix),
      toggle: () => setDraft(item.id, added ? withoutFixup(draft, fix.op) : withFixup(draft, fix)),
    };
  });
}
