import { Button } from '@maquette/ui';
import { useRecheckButton } from './RecheckButton.hooks';

export default function RecheckButton() {
  const { checking, detect } = useRecheckButton();
  return (
    <Button
      size="sm"
      icon="arrow-clockwise"
      loading={checking}
      tooltip="Looks for every tool on this computer again"
      onClick={() => void detect()}
    >
      Re-check all
    </Button>
  );
}
