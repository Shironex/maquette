import { useEffect, useMemo, useRef, useState } from 'react';
import { commands, errorMessage, type WorkspaceConfig } from '@/ipc/client';
import { useCatalog } from '@/features/providers/PresetFields';
import { useAppStore } from '@/state/app-store';
import { hasErrors, sameConfig, saveStatus, validateConfig } from '@/state/settings-form';

const DELAY_MS = 450;

/**
 * Keeps a local draft of the config and writes it back a moment after the
 * last valid change. Invalid drafts stay local until they are fixed.
 */
export function useAutosave() {
  const saved = useAppStore(s => s.workspace?.config);
  const setConfig = useAppStore(s => s.setConfig);
  const notify = useAppStore(s => s.notify);
  const catalog = useCatalog();
  const [edited, setDraft] = useState<WorkspaceConfig | undefined>(saved);
  // The switcher owns the current project, so a switch is no edit to save.
  const current = saved?.currentProject;
  const draft = useMemo(
    () => (edited && current !== undefined ? { ...edited, currentProject: current } : edited),
    [edited, current]
  );
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);
  /** The valid draft waiting for its timer, written at once if Settings closes first. */
  const pending = useRef<WorkspaceConfig | null>(null);
  const write = useRef<(config: WorkspaceConfig) => Promise<void>>(async () => undefined);

  write.current = async config => {
    pending.current = null;
    setSaving(true);
    try {
      const written = await commands.workspaceSaveConfig(config);
      setFailed(false);
      setConfig(written);
    } catch (error) {
      setFailed(true);
      notify({ status: 'failed', title: 'Could not save settings', detail: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  };

  useEffect(() => {
    pending.current = null;
    if (!draft || !saved || sameConfig(draft, saved) || hasErrors(validateConfig(draft))) return;
    pending.current = draft;
    const timer = window.setTimeout(() => void write.current(draft), DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [draft, saved]);

  useEffect(
    () => () => {
      if (pending.current) void write.current(pending.current);
    },
    []
  );

  const status = draft && saved ? saveStatus(draft, saved, { saving, failed }) : 'saved';
  return { draft, setDraft, status, errors: draft ? validateConfig(draft, catalog) : {} };
}
