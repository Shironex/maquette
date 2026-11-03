import { TextField } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import type { PlanPreset } from '@/ipc/client';
import { PresetPicker } from '../PresetPicker';
import { useNewCanvasDialog } from './NewCanvasDialog.hooks';

export interface NewCanvasDialogProps {
  onClose: () => void;
  /** The preset picked when it opens; Brainstorm (blank) by default. */
  preset?: PlanPreset;
}

/** Names a new canvas and picks the preset it starts from. It opens at once. */
export default function NewCanvasDialog({ onClose, preset = 'brainstorm' }: NewCanvasDialogProps) {
  const v = useNewCanvasDialog(onClose, preset);
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
        className="mt-2 flex flex-col gap-3"
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
          // Enter creates: with radios in the form, a browser need not submit it on its own.
          onKeyDown={e => {
            if (e.key !== 'Enter' || e.nativeEvent.isComposing) return;
            e.preventDefault();
            void v.submit();
          }}
          hint="Saved as a file in plans/ in the project folder."
        />
        <PresetPicker value={v.preset} onChange={v.setPreset} />
        <p className="text-meta text-fg-mute">
          Presets only set the starting notes. Every canvas can hold every kind of node.
        </p>
      </form>
    </ChoiceDialog>
  );
}
