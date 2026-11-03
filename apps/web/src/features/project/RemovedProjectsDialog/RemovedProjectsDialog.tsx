import { Button } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useRemovedProjectsDialog } from './RemovedProjectsDialog.hooks';

/** Removed projects: what waits in the workspace trash, each with Restore. */
export default function RemovedProjectsDialog() {
  const v = useRemovedProjectsDialog();
  if (!v) return null;
  const rows = v.rows.map(r => (
    <li
      key={r.trash}
      className="flex items-center gap-3 rounded-control border border-line px-3 py-2"
    >
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate font-semibold text-fg">{r.name}</span>
        <span className="truncate text-meta text-fg-mute">{r.meta}</span>
      </span>
      <Button
        size="sm"
        icon="arrow-counter-clockwise"
        disabled={v.busy}
        disabledReason="A restore is on its way"
        aria-label={`Restore ${r.name}`}
        onClick={() => v.restore(r.trash)}
      >
        Restore
      </Button>
    </li>
  ));
  return (
    <ChoiceDialog
      title="Removed projects"
      onDismiss={v.close}
      choices={[{ label: 'Close', onClick: v.close }]}
    >
      {v.rows.length === 0 ? (
        <p>
          No removed projects. Remove project in the switcher or in Settings &gt; Project moves one
          here.
        </p>
      ) : (
        <>
          <p>Restore moves the project folder back and opens it.</p>
          <ul
            aria-label="Removed projects"
            className="mt-2 flex max-h-[320px] flex-col gap-1 overflow-y-auto"
          >
            {rows}
          </ul>
        </>
      )}
    </ChoiceDialog>
  );
}
