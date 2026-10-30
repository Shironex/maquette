import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAvatarStore } from '@/state/use-avatar-store';
import type { ZoomMode } from '../SheetZoom/SheetZoom';
import type { ReweightReport } from '@/ipc/client';

const count = (n: number | undefined) => (n ?? 0).toLocaleString('en-US');

/**
 * The faces fused between an arm and the body that still stretch when it lifts, per side:
 * " 35 faces joining the right arm to the body still stretch when lifted.".
 */
function bridgesNote(report: ReweightReport): string {
  const sides = new Map<string, number>();
  for (const { side = '', faces = 0 } of report.bridges ?? []) {
    sides.set(side, (sides.get(side) ?? 0) + faces);
  }
  const parts = [...sides].map(
    ([side, faces]) => `${count(faces)} ${faces === 1 ? 'face' : 'faces'} joining the ${side} arm`
  );
  if (parts.length === 0) return '';
  const many = [...sides.values()].reduce((a, b) => a + b, 0) !== 1;
  return ` ${parts.join(' and ')} to the body still ${many ? 'stretch' : 'stretches'} when lifted.`;
}

/**
 * What a reweight moved, in a sentence: how many vertices, and from which arm bones most; and
 * the faces fused between an arm and the body that still stretch, when there are any.
 */
export function reweightSummary(report: ReweightReport): string {
  const from = Object.entries(report.fromBones ?? {})
    .sort((a, b) => b[1] - a[1])
    .slice(0, 2)
    .map(([bone]) => bone);
  const most = from.length > 0 ? `, most from ${from.join(' and ')}` : '';
  return (
    `Moved ${count(report.moved)} vertices of cloth and hair to the trunk${most}; ` +
    `kept ${count(report.keptAsSleeve)} on the arms as sleeve.` +
    bridgesNote(report)
  );
}

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
