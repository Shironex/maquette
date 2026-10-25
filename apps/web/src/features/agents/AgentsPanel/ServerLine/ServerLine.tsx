import { Icon, cn } from '@maquette/ui';
import type { McpStatus } from '@/ipc/client';
import { serverLine } from '@/state/agents';

export interface ServerLineProps {
  status: McpStatus | null;
  enabled: boolean;
}

export default function ServerLine({ status, enabled }: ServerLineProps) {
  const line = serverLine(status, enabled);
  return (
    <p
      role="status"
      className={cn(
        'flex items-center gap-1.5 text-meta',
        line.tone === 'ok' && 'text-sage',
        line.tone === 'error' && 'text-danger',
        line.tone === 'off' && 'text-fg-mute'
      )}
    >
      {line.tone === 'ok' ? (
        <span className="size-2 rounded-full bg-sage shadow-[0_0_0_3px_var(--color-sage-soft)]" />
      ) : (
        <Icon name={line.tone === 'error' ? 'warning-circle' : 'info'} size={14} />
      )}
      {line.text}
    </p>
  );
}
