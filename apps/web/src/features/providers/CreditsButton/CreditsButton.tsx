import { Button } from '@maquette/ui';
import { useCreditsButton } from './CreditsButton.hooks';

export interface CreditsButtonProps {
  /** Filters the sheet to this project; null shows every project. */
  project?: string | null;
  /** Runs before the sheet opens, for example to close the 3D queue. */
  onOpen?: () => void;
}

/** "Credits": opens the balances and the ledger of paid sends. */
export default function CreditsButton({ project = null, onOpen }: CreditsButtonProps) {
  const open = useCreditsButton(project, onOpen);
  return (
    <Button
      size="sm"
      variant="ghost"
      icon="gauge"
      tooltip="Balances, and what paid sends spent"
      onClick={open}
    >
      Credits
    </Button>
  );
}
