import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { itemTitle } from '@/state/inbox';
import { useDiscardDialog } from './DiscardDialog.hooks';
import type { DiscardDialogProps } from './DiscardDialog.types';

/**
 * Asks before Discard: says the file is moved aside, not deleted, and which
 * send closes with it. Cancel has the focus.
 */
export default function DiscardDialog(props: DiscardDialogProps) {
  const { item, onClose } = props;
  const { busy, confirm } = useDiscardDialog(props);
  return (
    <ChoiceDialog
      title={`Discard ${itemTitle(item)}?`}
      onDismiss={onClose}
      choices={[
        { label: 'Discard', variant: 'danger', loading: busy, onClick: () => void confirm() },
        { label: 'Cancel', onClick: onClose },
      ]}
    >
      <p>
        {item.id} leaves the Inbox. It is not deleted: the file moves into the workspace&apos;s
        inbox/.discarded folder, where you can still get it back.
      </p>
      {item.link ? (
        <p className="mt-2">
          The send {item.link.job} for {item.link.assetName} is closed as cancelled. Its earlier
          versions stay.
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
