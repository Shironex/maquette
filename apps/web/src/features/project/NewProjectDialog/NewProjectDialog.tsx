import { NewProjectSheet } from './NewProjectSheet';
import { useNewProjectDialog } from './NewProjectDialog.hooks';

export interface NewProjectDialogProps {
  onClosed?: () => void;
}

/** New project and Import folder from the switcher: the onboarding form, compact. */
export default function NewProjectDialog({ onClosed }: NewProjectDialogProps) {
  const mode = useNewProjectDialog();
  if (!mode) return null;
  // Keyed on the mode it opened with, so each open starts from a clean form.
  return <NewProjectSheet key={mode} initialMode={mode} onClosed={onClosed} />;
}
