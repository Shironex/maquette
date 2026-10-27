import { Button, IconButton, type ButtonSize } from '@maquette/ui';
import { useStopServer } from './StopServerButton.hooks';

export interface StopServerButtonProps {
  /** The project whose site it stops; nothing shows while another project's runs. */
  project: string | null;
  size?: ButtonSize;
  /** Only the stop icon, for a tight header; the tooltip names it. */
  compact?: boolean;
  className?: string;
}

/**
 * Stops the one site dev server that Preview on site, the Page view and agents
 * share. Shows only while it runs or starts for the project.
 */
export default function StopServerButton({
  project,
  size,
  compact,
  className,
}: StopServerButtonProps) {
  const { live, stopping, tooltip, detail, stop } = useStopServer(project);
  if (!live) return null;
  if (compact) {
    return (
      <IconButton
        icon={stopping ? 'circle-notch' : 'stop'}
        label="Stop the site's dev server"
        hint={detail}
        variant="secondary"
        disabled={stopping}
        className={className}
        onClick={() => void stop()}
      />
    );
  }
  return (
    <Button
      icon="stop"
      size={size}
      loading={stopping}
      tooltip={tooltip}
      className={className}
      onClick={() => void stop()}
    >
      Stop
    </Button>
  );
}
