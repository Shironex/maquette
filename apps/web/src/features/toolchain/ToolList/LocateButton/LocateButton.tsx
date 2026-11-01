import { Button } from '@maquette/ui';
import type { ToolId } from '@/ipc/client';

export interface LocateButtonProps {
  tool: ToolId;
  label: string;
  quiet: boolean;
  disabled: boolean;
  onLocate: (tool: ToolId) => Promise<void>;
  fixedWidth: boolean;
}

export default function LocateButton({
  tool,
  label,
  quiet,
  disabled,
  onLocate,
  fixedWidth,
}: LocateButtonProps) {
  return (
    <Button
      size="sm"
      variant={quiet ? 'ghost' : 'secondary'}
      disabled={disabled}
      aria-label={`Locate ${label}`}
      tooltip={`Pick where ${label} is installed on this computer`}
      className={fixedWidth ? 'w-[72px]' : undefined}
      onClick={() => void onLocate(tool)}
    >
      Locate...
    </Button>
  );
}
