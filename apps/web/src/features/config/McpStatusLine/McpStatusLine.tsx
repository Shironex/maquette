import { Icon, cn } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';

export interface McpStatusLineProps {
  config: WorkspaceConfig;
  className?: string;
}

/** The server line in onboarding, before the workspace (and the token) exists. */
export default function McpStatusLine({ config, className }: McpStatusLineProps) {
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
        ? `Starts on 127.0.0.1:${mcpPort} once the workspace exists. Copy the config, with its token, from Settings > Agents.`
        : 'Off. You can turn it on later in Settings > Agents.'}
    </p>
  );
}
