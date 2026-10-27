/**
 * Runs component tests against the mock backend, the same one `pnpm
 * dev:web` uses, through Tauri's IPC mock.
 */
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { commands } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import { events } from '@/ipc/generated/bindings';
import { useAppStore } from '@/state/use-app-store';
import { useUpdaterStore } from '@/state/use-updater-store';
import { INITIAL_UPDATER } from '@/state/updater';

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
  game?: string,
  library?: string,
  ship?: string,
  pipeline?: string
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
    library,
    ship,
    pipeline,
  });
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>));
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  return handle;
}

/** The configured workspace with a game project open: `crypt`, `import` or `maze`. */
export function openMockGame(game: 'crypt' | 'import' | 'maze' | 'hero') {
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

/** The configured workspace with the library: `demo` entries, or empty without. */
export function openMockLibrary(library?: 'demo') {
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
    undefined,
    library
  );
}

/** The configured workspace, with Frostveil set to ship as a hand-built site or not. */
export function openMockShip(ship?: 'hand-built') {
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
    undefined,
    undefined,
    ship
  );
}

/** The configured workspace with Moon Island's runner listing 24 models that skip the bake. */
export function openMockPipeline(pipeline: 'many') {
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
    undefined,
    undefined,
    undefined,
    pipeline
  );
}

export { clearMocks };

/**
 * The configured workspace with the app updater: inert without `updater`,
 * else on in that state (`available`, `ready`, `current` or `pending`, see
 * create-app-handlers.ts). Events reach the listeners, so download progress
 * shows. Resets the updater store.
 */
export async function openMockUpdater(updater?: string) {
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    updater,
    emit: event => {
      if (event.name === 'updater') void events.updaterUpdate.emit(event.payload);
    },
  });
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
    shouldMockEvents: true,
  });
  useUpdaterStore.setState({ ...INITIAL_UPDATER, refused: null, aboutAsked: false });
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  return handle;
}
