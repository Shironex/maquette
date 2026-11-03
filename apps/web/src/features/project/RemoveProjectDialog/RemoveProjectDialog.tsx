import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { useRemoveProjectDialog } from './RemoveProjectDialog.hooks';

/**
 * Asks before Remove project: names it, says its folder goes to the
 * workspace trash and can come back, and that the repo it ships to stays.
 * Work running in it refuses the remove; the core says what to wait for,
 * and Remove project stays greyed out with that reason.
 */
export default function RemoveProjectDialog() {
  const v = useRemoveProjectDialog();
  if (!v) return null;
  const { project } = v;
  const repo = v.game ? 'game repo' : 'site repo';
  return (
    <ChoiceDialog
      title={`Remove ${project.name}?`}
      onDismiss={v.cancel}
      choices={[
        {
          label: 'Remove project',
          variant: 'danger',
          loading: v.busy,
          disabled: v.refusedBecause !== null,
          disabledReason: v.refusedBecause ?? undefined,
          onClick: () => void v.confirm(),
        },
        { label: 'Cancel', onClick: v.cancel },
      ]}
    >
      <p>
        Its folder, with every asset, ref, model, canvas and run, moves to the workspace trash.
        Removed projects in the project switcher puts it back.
      </p>
      <p className="mt-2">
        {v.repo ? (
          <>
            The {repo} <span className="font-semibold text-fg">{v.repo}</span> is never touched.
          </>
        ) : (
          `A ${repo} it ships to is never touched.`
        )}{' '}
        Its sends keep their records.
      </p>
      {v.running ? (
        <p role="note" className="mt-3 text-amber">
          Work is running in this project. Remove project works again once it is over or cancelled.
        </p>
      ) : null}
      {v.error ? (
        <p role="alert" className="mt-3 text-danger">
          {v.error}
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
