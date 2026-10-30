import { useState } from 'react';
import type { VrmSummary } from '@/features/viewer/VrmStage';
import { fileUrl } from '@/ipc/files';
import { reducedMotion } from '@/lib/motion';
import { useAvatarStore } from '@/state/use-avatar-store';

/** The total VRM 1.0 humanoid bones, and how many it requires. */
const ALL_BONES = 55;
const REQUIRED = 15;

/** Why the eyes stay still, or null when the look at moves them. */
function stillEyes(summary: VrmSummary | null): string | null {
  if (!summary || summary.lookAtMoves) return null;
  return summary.lookAt === 'none'
    ? 'This file has no look at, so the eyes stay still.'
    : 'This file drives its eyes with expressions and has no look expressions yet, so following the pointer moves nothing. Expressions come in a later stage.';
}

/**
 * The written `.vrm` of the open avatar, the idle and look at options, and
 * what the stage found once it loaded. Results from an earlier file are
 * dropped when the file changes.
 */
export function usePreviewPanel() {
  const view = useAvatarStore(s => s.view);
  const setTab = useAvatarStore(s => s.setTab);
  const [armsDown, setArmsDown] = useState(true);
  const [breathe, setBreathe] = useState(() => !reducedMotion());
  const [lookAt, setLookAt] = useState(true);
  const [result, setResult] = useState<{
    url: string;
    summary: VrmSummary | null;
    failed: string | null;
  } | null>(null);
  const written = view?.export ?? null;
  const url = written && written.exists !== false ? fileUrl(written.path, written.writtenAt) : null;
  const current = result && result.url === url ? result : null;
  const summary = current?.summary ?? null;
  const roundTrip = written?.roundTrip ?? null;
  const chips = summary
    ? [
        `VRM ${summary.spec}`,
        `${summary.bones} of ${ALL_BONES} bones`,
        `${REQUIRED - summary.missing.length} of ${REQUIRED} required`,
        summary.springs > 0 ? `${summary.springs} spring joints` : 'No spring bones',
        summary.expressions > 0 ? `${summary.expressions} expressions` : 'No expressions',
      ]
    : [];
  return {
    view,
    written,
    url,
    name: view?.name ?? '',
    armsDown,
    setArmsDown,
    breathe,
    setBreathe,
    lookAt,
    setLookAt,
    summary,
    chips,
    failed: current?.failed ?? null,
    stillEyes: stillEyes(summary),
    missing: summary?.missing ?? [],
    roundTripProblems: roundTrip && !roundTrip.passed ? roundTrip.problems : null,
    onLoaded: (found: VrmSummary) => {
      if (url) setResult({ url, summary: found, failed: null });
    },
    onError: (message: string) => {
      if (url) setResult({ url, summary: null, failed: message });
    },
    toExport: () => setTab('export'),
  };
}
