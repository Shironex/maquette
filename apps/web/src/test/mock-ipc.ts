/**
 * Runs component tests against the mock backend, the same one `pnpm
 * dev:web` uses, through Tauri's IPC mock.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { commands } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import { useAppStore } from '@/state/use-app-store';

export async function openMockWorkspace(
  scenario = 'configured',
  projects?: string,
  handoff?: string,
  inbox?: string,
  arrival?: string,
  built?: string,
  motion?: string,
  page?: string,
  timeline?: string,
  paths?: string,
  game?: string
) {
  const handle = createBackend({
    scenario,
    probeDelay: 0,
    pace: 0,
    conceptMs: 60_000,
    projects,
    handoff,
    inbox,
    arrival,
    built,
    motion: motion ?? (timeline ? 'demo' : undefined),
    page,
    timeline,
    paths: paths ?? (timeline === 'demo' ? 'demo' : undefined),
    game,
  });
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>));
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  return handle;
}

/** The configured workspace with a game project open: `crypt`, `import` or `maze`. */
export function openMockGame(game: 'crypt' | 'import' | 'maze') {
  return openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    game
  );
}

export { clearMocks };
