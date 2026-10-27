import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { repoQuestion } from '@/state/sync';
import { useRepoPrompt } from './RepoPrompt.hooks';

/**
 * Asked once per project on a computer that does not know where the
 * project's repo is: it was picked on another computer sharing the
 * workspace, or the folder moved. "Not now" leaves it for Settings > Project.
 */
export default function RepoPrompt() {
  const { project, picking, error, choose, notNow } = useRepoPrompt();
  if (!project) return null;
  return (
    <ChoiceDialog
      title="Where is this project's repo on this computer?"
      onDismiss={notNow}
      choices={[
        { label: 'Not now', variant: 'ghost', onClick: notNow },
        {
          label: 'Choose folder',
          variant: 'primary',
          loading: picking,
          onClick: () => void choose(),
        },
      ]}
    >
      <p>{repoQuestion(project)}</p>
      <p className="mt-2 text-meta text-fg-mute">
        You can also set it later in Settings &gt; Project.
      </p>
      {error ? (
        <p role="alert" className="mt-2 text-meta text-danger">
          {error}
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
