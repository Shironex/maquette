import { TextField } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useLinkDialog } from './LinkDialog.hooks';

export interface LinkDialogProps {
  onAdd: (url: string, title: string) => void;
  onClose: () => void;
}

/** A web page for the canvas: its address and a title to show on the node. */
export default function LinkDialog({ onAdd, onClose }: LinkDialogProps) {
  const v = useLinkDialog(onAdd);
  return (
    <ChoiceDialog
      title="Add a link"
      onDismiss={onClose}
      choices={[
        { label: 'Cancel', onClick: onClose },
        {
          label: 'Add link',
          variant: 'primary',
          disabled: !v.canAdd,
          disabledReason: 'Type a web address first',
          onClick: v.submit,
        },
      ]}
    >
      <form
        className="mt-2 flex flex-col gap-3"
        onSubmit={e => e.preventDefault()}
        onKeyDown={e => {
          if (e.key === 'Enter') v.submit();
        }}
      >
        <TextField
          id={v.field}
          label="Address"
          placeholder="https://example.com/article"
          mono
          value={v.url}
          maxLength={2048}
          note={v.note}
          onChange={e => v.setUrl(e.target.value)}
          onBlur={v.touch}
        />
        <TextField
          label="Title"
          placeholder="What the page is about"
          value={v.title}
          maxLength={300}
          onChange={e => v.setTitle(e.target.value)}
        />
      </form>
    </ChoiceDialog>
  );
}
