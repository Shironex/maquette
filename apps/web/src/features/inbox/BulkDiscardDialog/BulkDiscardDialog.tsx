import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import type { InboxItem } from '@/ipc/client';
import { itemTitle } from '@/state/inbox';

export interface BulkDiscardDialogProps {
  /** The models to discard. */
  items: readonly InboxItem[];
  busy: boolean;
  onConfirm: () => void;
  onClose: () => void;
}

/**
 * Asks before discarding several models at once: they move aside, not
 * deleted, and the sends waiting on them close. Cancel has the focus.
 */
export default function BulkDiscardDialog({
  items,
  busy,
  onConfirm,
  onClose,
}: BulkDiscardDialogProps) {
  const n = items.length;
  const sends = items.filter(i => i.link).length;
  const names = items.slice(0, 4).map(itemTitle).join(', ');
  const more = n > 4 ? ` and ${n - 4} more` : '';
  return (
    <ChoiceDialog
      title={`Discard ${n} ${n === 1 ? 'model' : 'models'}?`}
      onDismiss={onClose}
      choices={[
        { label: `Discard ${n}`, variant: 'danger', loading: busy, onClick: onConfirm },
        { label: 'Cancel', onClick: onClose },
      ]}
    >
      <p>
        {names}
        {more} leave the Inbox. They are not deleted: the files move into the workspace&apos;s
        inbox/.discarded folder, and Restore puts one back.
      </p>
      {sends ? (
        <p className="mt-2">
          {sends === 1 ? 'The send' : `The ${sends} sends`} waiting on them{' '}
          {sends === 1 ? 'is' : 'are'} closed as cancelled. Earlier versions stay.
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
