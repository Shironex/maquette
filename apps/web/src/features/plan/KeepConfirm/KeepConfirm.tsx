import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useKeepConfirm } from './KeepConfirm.hooks';

/**
 * Asks before a Keep takes earlier agent changes along: each sits under
 * the one kept on the same parts of the canvas, and keeping only the newer
 * would let the older one's undo drop the person's words later. Nothing
 * is kept until the person says Keep all.
 */
export default function KeepConfirm() {
  const v = useKeepConfirm();
  if (!v.needs) return null;
  const items = v.needs.map(need => (
    <li key={need.id}>
      <span className="text-fg">{need.summary}</span>{' '}
      <span className="text-fg-mute">
        ({need.parts.join(', ')}, {v.when(need.at)}, {need.id})
      </span>
    </li>
  ));
  return (
    <ChoiceDialog
      title="Keep the earlier changes too?"
      onDismiss={v.cancel}
      choices={[
        { label: 'Cancel', onClick: v.cancel },
        { label: 'Keep all', variant: 'primary', onClick: v.keepAll },
      ]}
    >
      <p>
        Earlier agent changes sit under this one on the same parts of the canvas. Keeping this also
        keeps:
      </p>
      <ul className="mt-2 flex list-disc flex-col gap-1 pl-5">{items}</ul>
    </ChoiceDialog>
  );
}
