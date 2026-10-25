import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useQuitDialog } from './QuitDialog.hooks';

export default function QuitDialog() {
  const { asking, applying, error, cancel, applyAndQuit, quitWithout, names, title } =
    useQuitDialog();
  if (!asking) return null;
  const where = names.join(', ') || 'this workspace';
  return (
    <ChoiceDialog
      title={title}
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
      <p>Not applied yet in {where}. Quitting drops those edits.</p>
      {error ? (
        <p role="alert" className="mt-2 text-danger">
          Could not apply them, so Maquette is still open. {error}
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
