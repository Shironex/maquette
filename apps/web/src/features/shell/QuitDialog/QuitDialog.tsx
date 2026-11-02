import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useQuitDialog } from './QuitDialog.hooks';

export default function QuitDialog() {
  const {
    asking,
    applying,
    error,
    cancel,
    applyAndQuit,
    quitWithout,
    names,
    title,
    drafts,
    canvas,
  } = useQuitDialog();
  if (!asking) return null;
  const where = names.join(', ') || 'this workspace';
  const models = drafts === 1 ? '1 Inbox model has' : `${drafts} Inbox models have`;
  const saves = names.length > 0 || canvas !== null;
  return (
    <ChoiceDialog
      title={title}
      onDismiss={cancel}
      choices={[
        {
          label: names.length ? 'Quit without applying' : 'Quit anyway',
          variant: 'danger',
          onClick: () => void quitWithout(),
        },
        { label: 'Cancel', onClick: cancel },
        ...(saves
          ? [
              {
                label: names.length ? 'Apply and quit' : 'Save and quit',
                variant: 'primary' as const,
                loading: applying,
                onClick: () => void applyAndQuit(),
              },
            ]
          : []),
      ]}
    >
      {names.length ? <p>Not applied yet in {where}. Quitting drops those edits.</p> : null}
      {drafts ? (
        <p className={names.length ? 'mt-2' : undefined}>
          {models} a real size or fix-ups set. Accept saves them; quitting drops them.
        </p>
      ) : null}
      {canvas ? (
        <p className={names.length || drafts ? 'mt-2' : undefined}>
          The canvas {canvas} has edits that are not saved yet. Quitting without saving drops them.
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="mt-2 text-danger">
          Could not apply them, so Maquette is still open. {error}
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
