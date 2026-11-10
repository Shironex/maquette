/** Test support: the mock workspace mid agent session, with its MCP status loaded. */
import { mockIPC } from '@tauri-apps/api/mocks';
import { commands } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';

export async function openAgentSession(
  agents: 'active' | 'batch' | 'images' | 'preview' | 'many' | null = 'batch',
  /** `swapped`: a built asset took Crystal lantern's place and no Ship wrote it yet. */
  built: string | null = null
) {
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    conceptMs: 60_000,
    agents,
    built,
  });
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    return handle(cmd, (args ?? {}) as Record<string, unknown>);
  });
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  await useAgentsStore.getState().load();
  return calls;
}
