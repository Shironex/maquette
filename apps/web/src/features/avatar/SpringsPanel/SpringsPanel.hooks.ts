import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import {
  chainSummary,
  confirmedSprings,
  springsChoice,
  unmatchedSprings,
} from '@/state/avatar-springs';
import { useAvatarStore } from '@/state/use-avatar-store';
import type { SheetKind } from './SpringSheets';

/**
 * The Springs tab: the parts the finder proposed on the model the VRM is
 * written from, which of them the draft confirms, the chain step's state
 * and sheets, and the actions on them.
 */
export function useSpringsPanel() {
  const view = useAvatarStore(s => s.view);
  const draft = useAvatarStore(s => s.draft);
  const finding = useAvatarStore(s => s.finding);
  const building = useAvatarStore(s => s.building);
  const deciding = useAvatarStore(s => s.deciding);
  const confirm = useAvatarStore(s => s.confirmSpring);
  const edit = useAvatarStore(s => s.editSpring);
  const [zoom, setZoom] = useState<SheetKind | null>(null);
  const now = useNow(60_000);
  const springs = view?.springs ?? null;
  const found = springs?.found ?? null;
  const chained = springs?.chained ?? null;
  const confirmed = confirmedSprings(draft);
  const candidates = found?.candidates ?? [];
  const pending = chained?.state === 'pending';
  const busy = finding || building || deciding;
  return {
    view,
    ready: Boolean(springs?.ready),
    found,
    candidates,
    confirmed,
    /** Picked springs, none confirmed, or nothing picked yet. */
    choice: springsChoice(draft),
    /** Confirmed springs no candidate goes by now. */
    unmatched: found ? unmatchedSprings(draft, candidates) : [],
    /** The last VRM written carries springs. */
    hadSprings: (view?.export?.springs ?? 0) > 0,
    chained,
    state: chained?.state ?? null,
    summary: chained ? chainSummary(chained) : null,
    approvedAgo: chained?.approvedAt ? formatAgo(Date.parse(chained.approvedAt), now) : null,
    sheets: {
      candidates: fileUrl(found?.sheet ?? null),
      before: fileUrl(chained?.sheets.before ?? null),
      after: fileUrl(chained?.sheets.after ?? null),
    },
    /** Which Blender step runs now. */
    status: { finding, building, deciding },
    busy,
    /** Why Make chains cannot run yet, or null. */
    makeBlocked:
      confirmed.length === 0
        ? 'Confirm at least one part first'
        : pending
          ? 'Approve or deny the chains first'
          : null,
    actions: {
      find: () => void useAvatarStore.getState().findSprings(),
      confirm,
      edit,
      none: (on: boolean) => useAvatarStore.getState().confirmNoSprings(on),
      drop: (id: string) => useAvatarStore.getState().dropSpring(id),
      make: () => void useAvatarStore.getState().build(),
      approve: () => void useAvatarStore.getState().decideSprings(true),
      deny: () => void useAvatarStore.getState().decideSprings(false),
      toExport: () => useAvatarStore.getState().setTab('export'),
    },
    /** The sheet shown up close, or null. */
    zoom: { kind: zoom, open: (kind: SheetKind) => setZoom(kind), close: () => setZoom(null) },
  };
}
