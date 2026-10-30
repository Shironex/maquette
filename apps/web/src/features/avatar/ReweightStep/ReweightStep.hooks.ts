import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { reweightSummary } from '@/state/avatar';
import { useAvatarStore } from '@/state/use-avatar-store';
import type { ZoomMode } from '../SheetZoom/SheetZoom';

/** The step that frees cloth and hair from the arms, before the arms are lifted. */
export function useReweightStep() {
  const view = useAvatarStore(s => s.view);
  const deciding = useAvatarStore(s => s.deciding);
  const building = useAvatarStore(s => s.building);
  const decide = useAvatarStore(s => s.decideReweight);
  const [zoom, setZoom] = useState<ZoomMode | null>(null);
  const now = useNow(60_000);
  const r = view?.reweighted ?? null;
  return {
    show: Boolean(view?.needsReweight || r),
    reweighted: r,
    state: r?.state ?? null,
    finding: view?.readiness?.findings.find(f => f.id === 'arm-weights') ?? null,
    summary: r ? reweightSummary(r.report) : null,
    approvedAgo: r?.approvedAt ? formatAgo(Date.parse(r.approvedAt), now) : null,
    sheets: r ? { before: fileUrl(r.sheets.before), after: fileUrl(r.sheets.after) } : null,
    busy: deciding || building,
    deciding,
    approve: () => void decide(true),
    deny: () => void decide(false),
    zoom,
    openZoom: (mode: ZoomMode) => setZoom(mode),
    closeZoom: () => setZoom(null),
  };
}
