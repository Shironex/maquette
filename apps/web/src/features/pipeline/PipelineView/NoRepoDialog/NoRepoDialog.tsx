import { ChoiceDialog } from '@/features/common/ChoiceDialog';

export interface NoRepoDialogProps {
  /** "site" or "game". */
  repo: string;
  onRun: () => void;
  onSettings: () => void;
  onDismiss: () => void;
}

/** Asks before a run that cannot ship: it says so before the time is spent. */
export default function NoRepoDialog({ repo, onRun, onSettings, onDismiss }: NoRepoDialogProps) {
  return (
    <ChoiceDialog
      title={`Run without a ${repo} repo?`}
      onDismiss={onDismiss}
      choices={[
        { label: 'Open Settings', variant: 'ghost', onClick: onSettings },
        { label: 'Run, ship nothing', variant: 'primary', onClick: onRun },
      ]}
    >
      <p>
        This project has no {repo} repo, so the run builds everything and ships nothing. Pick one in
        Settings to ship, or tick Build only to skip this question.
      </p>
    </ChoiceDialog>
  );
}
