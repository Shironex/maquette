import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { measureRows, measuresOf, poseSummary } from '@/state/avatar';
import { useAvatarStore } from '@/state/use-avatar-store';
import type { ZoomMode } from '../SheetZoom/SheetZoom';

/** The rest pose as it is, or as the last normalise left it and its sheets. */
export function useRestPosePanel() {
  const view = useAvatarStore(s => s.view);
  const elbows = useAvatarStore(s => s.elbows);
  const setElbows = useAvatarStore(s => s.setElbows);
  const fingers = useAvatarStore(s => s.fingers);
  const setFingers = useAvatarStore(s => s.setFingers);
  const deciding = useAvatarStore(s => s.deciding);
  const building = useAvatarStore(s => s.building);
  const decide = useAvatarStore(s => s.decide);
  const [zoom, setZoom] = useState<ZoomMode | null>(null);
  const now = useNow(60_000);
  const n = view?.normalized ?? null;
  const before = n?.report.before ?? measuresOf(view?.readiness ?? null);
  return {
    view,
    /** Freeing cloth and hair from the arms comes first. */
    stepped: Boolean(view?.needsReweight || view?.reweighted),
    normalized: n,
    state: n?.state ?? null,
    rows: measureRows(before ?? null, n?.report.after ?? null),
    summary: n ? poseSummary(n.report) : null,
    approvedAgo: n?.approvedAt ? formatAgo(Date.parse(n.approvedAt), now) : null,
    sheets: n ? { before: fileUrl(n.sheets.before), after: fileUrl(n.sheets.after) } : null,
    findingsAfter: n?.readiness.findings.filter(f => f.level !== 'note') ?? [],
    elbows,
    setElbows,
    fingers,
    setFingers,
    busy: deciding || building,
    deciding,
    approve: () => void decide(true),
    deny: () => void decide(false),
    zoom,
    openZoom: (mode: ZoomMode) => setZoom(mode),
    closeZoom: () => setZoom(null),
  };
}
