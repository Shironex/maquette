import { useState } from 'react';
import type { InboxItem, ProjectManifest } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import type { PreviewMode } from '@/features/viewer/ModelPreview';
import { useNow } from '@/lib/useNow';
import { acceptTarget, madeBy, sentFor, sortChecks } from '@/state/inbox';
import { PROVIDER_LABEL } from '@/state/providers';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { MODES } from './constants';
import { sourceItems } from './sourceItems';

const NO_PROJECTS: readonly ProjectManifest[] = [];

/** The project an accept goes to, what the item reads as and the inbox actions. */
export function useInboxDetail(item: InboxItem) {
  const now = useNow(30_000);
  const current = useAppStore(s => s.workspace?.project ?? null);
  const projects = useAppStore(s => s.workspace?.projects) ?? NO_PROJECTS;
  const { busy, reprocess, accept, openInBlender, setKind } = useInboxStore();
  const project = acceptTarget(item, projects, current);
  const sentLabel = item.link
    ? sentFor(item, projects, current?.id ?? null, PROVIDER_LABEL[item.link.provider])
    : null;
  const maker = madeBy(item);
  const acceptTitle = !project
    ? 'Create a project first'
    : item.link && item.link.project === project.id
      ? `Adds a version to ${item.link.assetName}`
      : `Moves it into ${project.name}`;
  return {
    now,
    project,
    maker,
    checks: sortChecks(item.checks),
    source: sourceItems(item, sentLabel, maker),
    isBusy: busy === item.id,
    acceptTitle,
    reprocess,
    accept,
    openInBlender,
    setKind,
  };
}

/** The viewport's mode and the error the preview reported for this file. */
export function useInboxPreview(item: InboxItem) {
  const [mode, setMode] = useState<PreviewMode>('shaded');
  const [loadError, setLoadError] = useState<string | null>(null);
  const url = fileUrl(item.file);
  const hasBones = (item.report?.joints ?? 0) > 0;
  return {
    mode,
    setMode,
    loadError,
    setLoadError,
    url,
    modes: MODES.filter(m => m.value !== 'bones' || hasBones),
    modeLabel: MODES.find(m => m.value === mode)?.label,
    noPreview: !url || loadError || item.state === 'failed',
    orbitHint: url && item.state === 'ready' && !loadError,
  };
}
