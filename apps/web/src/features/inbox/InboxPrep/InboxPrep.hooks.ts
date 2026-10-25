import { useEffect } from 'react';
import type { InboxItem, ModelPrep, ProjectManifest } from '@/ipc/client';
import { vec3 } from '@/state/inspector';
import { EMPTY_PREP, previewKey, workspaceDefaults } from '@/state/prepare';
import { useAppStore } from '@/state/use-app-store';
import { previewTarget, suggestionKey, usePrepareStore } from '@/state/use-prepare-store';

/**
 * An inbox model's draft real size and fix-ups: kept in the store until
 * Accept saves them with the new version, with the suggestion for its name
 * and kind and the preview of the draft on this file.
 */
export function useInboxPrep(item: InboxItem, project: ProjectManifest | null) {
  const draft = usePrepareStore(s => s.drafts[item.id]) ?? EMPTY_PREP;
  const setDraft = usePrepareStore(s => s.setDraft);
  const suggest = usePrepareStore(s => s.suggest);
  const run = usePrepareStore(s => s.preview);
  const suggestion = usePrepareStore(s => s.suggestions[suggestionKey(item.name, item.kind)]);
  const target = { project: project?.id ?? null, asset: null, inbox: item.id };
  const preview = usePrepareStore(s => s.previews[previewKey(previewTarget(target), draft)]);
  const config = useAppStore(s => s.workspace?.config);
  useEffect(() => {
    void suggest(item.name, item.kind);
  }, [suggest, item.name, item.kind]);
  const readable = item.state === 'ready' && item.report !== null;
  return {
    draft,
    onChange: (next: ModelPrep) => setDraft(item.id, next),
    measured: item.report ? vec3(item.report.size) : null,
    suggestion: suggestion ?? null,
    defaults: workspaceDefaults(config),
    preview: preview ?? null,
    onPreview: readable ? () => void run({ ...target, plan: draft }) : null,
  };
}
