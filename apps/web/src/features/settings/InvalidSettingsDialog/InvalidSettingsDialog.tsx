import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useInvalidSettingsDialog } from './InvalidSettingsDialog.hooks';
import type { InvalidSettingsDialogProps } from './InvalidSettingsDialog.types';

/** Asks before Settings closes over a field that is not valid. */
export default function InvalidSettingsDialog({
  leaveTo,
  problems,
  discardInvalid,
}: InvalidSettingsDialogProps) {
  const { stay, discard } = useInvalidSettingsDialog(leaveTo, discardInvalid);
  const lines = problems.map(line => <li key={line}>{line}</li>);
  return (
    <ChoiceDialog
      title="Some settings are not valid"
      onDismiss={stay}
      choices={[
        { label: 'Discard', variant: 'danger', onClick: discard },
        { label: 'Fix it', variant: 'primary', onClick: stay },
      ]}
    >
      <ul className="m-0 flex list-none flex-col gap-1 p-0">{lines}</ul>
      <p className="mt-2 text-meta text-fg-mute">
        Discard puts those fields back to what is saved. Your other changes are kept.
      </p>
    </ChoiceDialog>
  );
}
