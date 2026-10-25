import type { McpStatus } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { useAgentsStore } from '@/state/use-agents-store';

/** The calls to list, refreshed every 30 s so their times stay current. */
export function useActivityList(status: McpStatus | null) {
  const clear = useAgentsStore(s => s.clearActivity);
  const calls = status?.calls ?? [];
  useNow(30_000);
  return { calls, clear };
}
