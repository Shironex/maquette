/**
 * Opens the mock workspace for the Avatar view (`?avatar=` in the browser)
 * and loads one character asset into the Avatar store.
 */
import { mockIPC } from '@tauri-apps/api/mocks';
import { commands } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { createBackend } from '@/ipc/mock/backend';
import { avatarSheetFiles, avatarVrmFiles } from '@/ipc/mock/create-avatar-handlers';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';

/** Every command since the last open, with its arguments. */
export const avatarCalls: { cmd: string; args: Record<string, unknown> }[] = [];

export async function openAvatar(
  avatar: 'pending' | 'written' | 'round-trip' | 'older' | 'custom' | null = null,
  asset = 'elsheria'
) {
  avatarCalls.length = 0;
  useAvatarStore.setState(useAvatarStore.getInitialState(), true);
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    conceptMs: 60_000,
    avatar,
  });
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    avatarCalls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    return handle(cmd, (args ?? {}) as Record<string, unknown>);
  });
  const boot = await commands.appBootstrap();
  const root = boot.workspace!.root;
  for (const [path, sheet] of Object.entries(avatarSheetFiles(root))) {
    registerMockFile(path, `/${sheet}`);
  }
  for (const [path, file] of Object.entries(avatarVrmFiles(root, ['frostveil']))) {
    registerMockFile(path, `/${file}`);
  }
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  await useAgentsStore.getState().load();
  useAvatarStore.setState({ project: 'frostveil', asset });
  await useAvatarStore.getState().load();
  return calls;
}

export { clearMocks } from './mock-ipc';
