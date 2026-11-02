import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import type { PlanSummary } from '@/ipc/client';
import { useRemoveCanvasDialog } from './RemoveCanvasDialog.hooks';

export interface RemoveCanvasDialogProps {
  plan: PlanSummary;
  onClose: () => void;
}

/**
 * Asks before Remove: says the file moves to the workspace trash, not
 * deleted, and that its pictures stay. Cancel has the focus.
 */
export default function RemoveCanvasDialog({ plan, onClose }: RemoveCanvasDialogProps) {
  const { busy, confirm } = useRemoveCanvasDialog(plan, onClose);
  return (
    <ChoiceDialog
      title={`Remove ${plan.name}?`}
      onDismiss={onClose}
      choices={[
        { label: 'Remove', variant: 'danger', loading: busy, onClick: () => void confirm() },
        { label: 'Cancel', onClick: onClose },
      ]}
    >
      <p>
        {plan.file} leaves the project. It is not deleted: the file moves into the workspace&apos;s
        trash folder, where you can still get it back.
      </p>
      <p className="mt-2">Pictures the canvas copied into plans/images stay where they are.</p>
    </ChoiceDialog>
  );
}
