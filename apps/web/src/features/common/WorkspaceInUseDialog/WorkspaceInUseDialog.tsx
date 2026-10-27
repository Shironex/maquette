import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useWorkspaceInUseDialog } from './WorkspaceInUseDialog.hooks';

/**
 * Asked before a workspace opens while another computer has it open. The
 * focus starts on Cancel, the answer that changes nothing.
 */
export default function WorkspaceInUseDialog() {
  const { question, openAnyway, cancel } = useWorkspaceInUseDialog();
  if (!question) return null;
  return (
    <ChoiceDialog
      title="Open this workspace here too?"
      onDismiss={cancel}
      choices={[
        { label: 'Open anyway', variant: 'danger', onClick: openAnyway },
        { label: 'Cancel', variant: 'primary', onClick: cancel },
      ]}
    >
      <p>{question.message}</p>
      <p className="mt-2 text-meta text-fg-mute">
        Maquette notices this from the lock the other computer keeps fresh. Open anyway when you are
        sure it is closed there, for example when that computer is off.
      </p>
    </ChoiceDialog>
  );
}
