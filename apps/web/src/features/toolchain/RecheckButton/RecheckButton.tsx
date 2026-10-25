import { Button } from '@maquette/ui';
import { useRecheckButton } from './RecheckButton.hooks';

export default function RecheckButton() {
  const { checking, detect } = useRecheckButton();
  return (
    <Button size="sm" icon="arrow-clockwise" loading={checking} onClick={() => void detect()}>
      Re-check all
    </Button>
  );
}
