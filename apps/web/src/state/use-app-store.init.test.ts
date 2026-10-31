import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';

afterEach(clearMocks);

describe('init', () => {
  it('shares one bootstrap while it is on its way, and can run again after', async () => {
    let calls = 0;
    mockIPC(cmd => {
      if (cmd !== 'app_bootstrap') return undefined;
      calls += 1;
      return { workspace: null, lastWorkspaceError: null, workspaceInUse: null };
    });
    const { init } = useAppStore.getState();
    await Promise.all([init(), init()]);
    expect(calls).toBe(1);
    await init();
    expect(calls).toBe(2);
  });
});
