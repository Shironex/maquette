import { useMemo, useRef, useState } from 'react';
import type { AssetKind, InspectView } from '@/ipc/client';
import type { SidePick } from '@/state/inspector';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { compareGroups } from './compare-options';

/** The room below the button the menu may take before it opens upwards instead. */
const MIN_BELOW = 260;
/** The gap kept between the menu and the window edge. */
const EDGE = 12;
/** The gap between the button and the menu (the `mt-1` of the popover). */
const GAP = 4;

export interface Placement {
  up: boolean;
  maxHeight: number;
}

/**
 * Whether the menu is open and where: below the button, or above it when
 * there is more room there, capped to the room in the window.
 */
export function useCompareMenu(view: InspectView, pick: SidePick) {
  const [open, setOpen] = useState(false);
  const [placement, setPlacement] = useState<Placement>({ up: false, maxHeight: 480 });
  const anchorRef = useRef<HTMLDivElement>(null);
  const choose = useInspectorStore(s => s.pick);
  const assets = useAppStore(s => s.workspace?.project?.assets);
  const kinds = useMemo(
    () => new Map<string, AssetKind>((assets ?? []).map(a => [a.id, a.kind])),
    [assets]
  );
  const groups = useMemo(
    () => compareGroups(view, pick, kinds, choose),
    [view, pick, kinds, choose]
  );

  const toggle = () => {
    if (!open && anchorRef.current) {
      const rect = anchorRef.current.getBoundingClientRect();
      const below = window.innerHeight - rect.bottom - EDGE - GAP;
      const above = rect.top - EDGE - GAP;
      const up = below < MIN_BELOW && above > below;
      setPlacement({ up, maxHeight: Math.max(160, Math.floor(up ? above : below)) });
    }
    setOpen(o => !o);
  };
  return { open, toggle, close: () => setOpen(false), groups, anchorRef, placement };
}
