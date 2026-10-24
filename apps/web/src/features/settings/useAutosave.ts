import { useEffect, useState } from 'react';
import { commands, errorMessage, type WorkspaceConfig } from '@/ipc/client';
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
  const [draft, setDraft] = useState<WorkspaceConfig | undefined>(saved);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!draft || !saved || sameConfig(draft, saved) || hasErrors(validateConfig(draft))) return;
    const timer = window.setTimeout(async () => {
      setSaving(true);
      try {
        const written = await commands.workspaceSaveConfig(draft);
        setFailed(false);
        setConfig(written);
      } catch (error) {
        setFailed(true);
        notify({ status: 'failed', title: 'Could not save settings', detail: errorMessage(error) });
      } finally {
        setSaving(false);
      }
    }, DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [draft, saved, setConfig, notify]);

  const status = draft && saved ? saveStatus(draft, saved, { saving, failed }) : 'saved';
  return { draft, setDraft, status, errors: draft ? validateConfig(draft) : {} };
}
