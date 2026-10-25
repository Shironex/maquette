import { useAgentsStore } from '@/state/use-agents-store';

/** The MCP server's status as the panel shows it. */
export function useAgentsPanel() {
  const status = useAgentsStore(s => s.status);
  return { status };
}
