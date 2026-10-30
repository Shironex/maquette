import { useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAvatarStore } from '@/state/use-avatar-store';
import type { ZoomMode } from '../SheetZoom/SheetZoom';
import type { BridgeFaces, ReweightReport } from '@/ipc/client';
import type { Choice } from '@/state/avatar';

const count = (n: number | undefined) => (n ?? 0).toLocaleString('en-US');

/** What the reweight does with the faces fused between an arm and the body. */
export const BRIDGES: readonly (Choice<BridgeFaces> & { hint: string })[] = [
  {
    value: 'hide',
    label: 'Hide',
    hint: 'Takes only those faces out of the model, so they are not drawn. Every vertex, UV, texture and weight stays.',
  },
  { value: 'keep', label: 'Keep', hint: 'Leaves them, stretching as the arm lifts.' },
];

/**
 * The faces fused between an arm and the body that stretch when it lifts, per side, and
 * whether they were hidden: " 35 faces joining the right arm to the body are hidden, as they
 * would stretch when lifted.".
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
  const total = [...sides.values()].reduce((a, b) => a + b, 0);
  const many = total !== 1;
  const joining = `${parts.join(' and ')} to the body`;
  const hidden = report.hiddenFaces ?? 0;
  if (hidden >= total) {
    return ` ${joining} ${many ? 'are' : 'is'} hidden, as ${many ? 'they' : 'it'} would stretch when lifted.`;
  }
  const still = ` ${joining} still ${many ? 'stretch' : 'stretches'} when lifted.`;
  return hidden > 0 ? `${still} ${count(hidden)} of them hidden.` : still;
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
  const setBridges = useAvatarStore(s => s.setBridges);
  const stored = useAvatarStore(s => s.view?.spec.bridges ?? 'hide');
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
    /** The choice shows once a reweight found faces fused between an arm and the body. */
    showBridges: (r?.report.bridges?.length ?? 0) > 0,
    bridges: r?.bridges ?? stored,
    bridgesHint: BRIDGES.find(b => b.value === (r?.bridges ?? stored))?.hint ?? '',
    setBridges: (bridges: BridgeFaces) => void setBridges(bridges),
    deciding,
    approve: () => void decide(true),
    deny: () => void decide(false),
    zoom,
    openZoom: (mode: ZoomMode) => setZoom(mode),
    closeZoom: () => setZoom(null),
  };
}
