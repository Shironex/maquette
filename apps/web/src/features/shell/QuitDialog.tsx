import { useEffect } from 'react';
import { commands } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { useAppStore } from '@/state/app-store';
import { usePipelineStore } from '@/state/pipeline-store';
import { unappliedProjects, useQuitStore } from '@/state/quit-store';
import { ChoiceDialog } from '../common/ChoiceDialog';

/**
 * Tells the shell whether presets wait to be applied, so a close only needs
 * the web view's answer then, and opens the question when the shell asks.
 */
export function useQuitGuard() {
  const waiting = usePipelineStore(s => unappliedProjects(s.presetEdits).length > 0);
  const ask = useQuitStore(s => s.ask);
  useEffect(() => {
    void commands.quitSetUnapplied(waiting).catch(() => undefined);
  }, [waiting]);
  useEffect(() => {
    const stop = events.quitRequested.listen(ask);
    return () => void stop.then(fn => fn()).catch(() => undefined);
  }, [ask]);
}

export function QuitDialog() {
  const { asking, applying, error } = useQuitStore();
  const cancel = useQuitStore(s => s.cancel);
  const applyAndQuit = useQuitStore(s => s.applyAndQuit);
  const quitWithout = useQuitStore(s => s.quitWithoutApplying);
  const edits = usePipelineStore(s => s.presetEdits);
  const projects = useAppStore(s => s.workspace?.projects);
  const names = unappliedProjects(edits).map(id => projects?.find(p => p.id === id)?.name ?? id);
  if (!asking) return null;
  return (
    <ChoiceDialog
      title="Apply the compression presets?"
      onDismiss={cancel}
      choices={[
        { label: 'Quit without applying', variant: 'danger', onClick: () => void quitWithout() },
        { label: 'Cancel', onClick: cancel },
        {
          label: 'Apply and quit',
          variant: 'primary',
          loading: applying,
          onClick: () => void applyAndQuit(),
        },
      ]}
    >
      <p>Not applied yet in {names.join(', ') || 'this workspace'}. Quitting drops those edits.</p>
      {error ? (
        <p role="alert" className="mt-2 text-danger">
          Could not apply them, so Maquette is still open. {error}
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
