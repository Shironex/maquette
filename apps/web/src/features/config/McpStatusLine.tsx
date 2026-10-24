import { Icon, cn } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';

/**
 * The server line under "Built-in MCP server". The server itself is not
 * built yet, so this never claims it is running.
 */
export function McpStatusLine({
  config,
  className,
}: {
  config: WorkspaceConfig;
  className?: string;
}) {
  const { mcpEnabled, mcpPort } = config.agents;
  return (
    <p
      className={cn(
        'flex items-center gap-1.5 text-meta',
        mcpEnabled ? 'text-sage' : 'text-fg-mute',
        className
      )}
    >
      <Icon name={mcpEnabled ? 'check-circle' : 'info'} size={14} />
      {mcpEnabled
        ? `Port ${mcpPort} reserved. The server starts here in a later version.`
        : 'Off. Turn it on to reserve a port for agents.'}
    </p>
  );
}
