import type { Check, InboxItem } from '@/ipc/client';
import { EMPTY_PREP, describeFixup, hasFixup, withFixup, withoutFixup } from '@/state/prepare';
import { usePrepareStore } from '@/state/use-prepare-store';
import { offersRigDoctor } from '@/state/rig';
import { useRigStore } from '@/state/use-rig-store';
import { useSplitStore } from '@/state/use-split-store';

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

/**
 * One mesh from a send: opens the split sheet on the Inbox model, so Tripo
 * or Meshy can find the parts before it is accepted. Null for other checks.
 */
export function useProviderSplit(item: InboxItem, check: Check): (() => void) | null {
  const openSplit = useSplitStore(s => s.openSplit);
  const link = item.link;
  if (check.id !== 'merged' || !link) return null;
  return () =>
    openSplit({ project: link.project, asset: link.asset, version: null, item: item.id });
}

/** A rig check that is not a pass: opens the rig doctor on the model. Null otherwise. */
export function useRigDoctorFix(item: InboxItem, check: Check): (() => void) | null {
  const show = useRigStore(s => s.show);
  if (!offersRigDoctor(check)) return null;
  return () => void show({ inbox: item.id, project: null, asset: null, version: null }, item.name);
}
