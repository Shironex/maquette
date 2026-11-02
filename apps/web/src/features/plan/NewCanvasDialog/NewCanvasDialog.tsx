import { TextField } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useNewCanvasDialog } from './NewCanvasDialog.hooks';

export interface NewCanvasDialogProps {
  onClose: () => void;
}

/** Names a new canvas. It starts empty and opens at once. */
export default function NewCanvasDialog({ onClose }: NewCanvasDialogProps) {
  const v = useNewCanvasDialog(onClose);
  return (
    <ChoiceDialog
      title="New canvas"
      onDismiss={onClose}
      choices={[
        { label: 'Cancel', onClick: onClose },
        {
          label: 'Create',
          variant: 'primary',
          loading: v.busy,
          disabled: !v.canCreate,
          disabledReason: 'Give the canvas a name first',
          onClick: () => void v.submit(),
        },
      ]}
    >
      <form
        className="mt-2 flex flex-col gap-2"
        onSubmit={e => {
          e.preventDefault();
          void v.submit();
        }}
      >
        <TextField
          id={v.field}
          label="Name"
          placeholder="Untitled canvas"
          value={v.name}
          maxLength={120}
          onChange={e => v.setName(e.target.value)}
          hint="Saved as a file in plans/ in the project folder."
        />
      </form>
    </ChoiceDialog>
  );
}
