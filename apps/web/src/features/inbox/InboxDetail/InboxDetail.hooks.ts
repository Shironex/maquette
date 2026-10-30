import { useState } from 'react';
import type { InboxItem, ProjectManifest } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import type { PreviewMode } from '@/features/viewer/ModelPreview';
import { reducedMotion } from '@/lib/motion';
import { useNow } from '@/lib/useNow';
import { acceptTarget, goesTo, madeBy, sentFor, sortChecks } from '@/state/inbox';
import { EMPTY_PREP, previewKey } from '@/state/prepare';
import { previewTarget, usePrepareStore } from '@/state/use-prepare-store';
import { PROVIDER_LABEL, jobChip } from '@/state/providers';
import { redoBlocker, redoUnderWay } from '@/state/redo';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { MODES } from '@/features/viewer/previewModes';
import { sourceItems } from './sourceItems';

const NO_PROJECTS: readonly ProjectManifest[] = [];

/** The project an accept goes to, what the item reads as and the inbox actions. */
export function useInboxDetail(item: InboxItem) {
  const now = useNow(30_000);
  const current = useAppStore(s => s.workspace?.project ?? null);
  const projects = useAppStore(s => s.workspace?.projects) ?? NO_PROJECTS;
  const { busy, reprocess, accept, openInBlender, setKind } = useInboxStore();
  const [discarding, setDiscarding] = useState(false);
  const openRedo = useProvidersStore(s => s.openRedo);
  const pending = redoUnderWay(
    useProvidersStore(s => s.jobs),
    item
  );
  const project = acceptTarget(item, projects, current);
  const goal = goesTo(item);
  const sentLabel = item.link
    ? sentFor(item, projects, current?.id ?? null, PROVIDER_LABEL[item.link.provider])
    : null;
  const maker = madeBy(item);
  const acceptTitle = !project
    ? 'Create a project first'
    : goal && goal.project === project.id
      ? `Adds a version to ${goal.assetName}`
      : `Moves it into ${project.name}`;
  return {
    now,
    project,
    maker,
    checks: sortChecks(item.checks),
    source: sourceItems(
      item,
      sentLabel,
      maker,
      pending ? `${pending.id}, ${jobChip(pending).label.toLowerCase()}` : null
    ),
    redoBlocked: redoBlocker(item),
    redo: () => item.link && void openRedo(item.link.project, item.link.asset, item.link.job),
    isBusy: busy === item.id,
    discarding,
    askDiscard: () => setDiscarding(true),
    closeDiscard: () => setDiscarding(false),
    acceptTitle,
    reprocess,
    accept,
    openInBlender,
    setKind,
  };
}

/**
 * The split a draft's preview made of this file, when one ran: the Parts
 * mode shows that copy, coloured by part.
 */
function useSplitPreview(item: InboxItem, project: string | null) {
  const draft = usePrepareStore(s => s.drafts[item.id]) ?? EMPTY_PREP;
  const target = previewTarget({ project, asset: null, inbox: item.id });
  const preview = usePrepareStore(s => s.previews[previewKey(target, draft)]);
  const split = draft.fixups.find(f => f.op === 'split-parts');
  if (!split || split.op !== 'split-parts' || preview?.status !== 'done') return null;
  return {
    url: fileUrl(preview.report.output),
    label: `Split preview · ${split.by === 'material' ? 'by material' : 'by loose pieces'} · on a copy`,
  };
}

/** The viewport's mode and the error the preview reported for this file. */
export function useInboxPreview(item: InboxItem, project: string | null = null) {
  const [mode, setMode] = useState<PreviewMode>('shaded');
  // It spins until stopped, so it starts still when the system asks for less motion.
  const [turning, setTurning] = useState(() => !reducedMotion());
  const [loadError, setLoadError] = useState<string | null>(null);
  const split = useSplitPreview(item, project);
  const own = fileUrl(item.file);
  const url = mode === 'parts' && split?.url ? split.url : own;
  const hasBones = (item.report?.joints ?? 0) > 0;
  const label = MODES.find(m => m.value === mode)?.label;
  return {
    mode,
    setMode,
    turning,
    toggleTurning: () => setTurning(on => !on),
    loadError,
    setLoadError,
    url,
    modes: MODES.filter(
      m => (m.value !== 'bones' || hasBones) && (m.value !== 'parts' || item.kind !== 'scene')
    ),
    modeLabel: mode === 'parts' ? (split?.label ?? 'Parts · as it came') : label,
    noPreview: !url || loadError || item.state === 'failed',
    orbitHint: url && item.state === 'ready' && !loadError,
  };
}
