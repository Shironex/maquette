import { afterEach, describe, expect, it } from 'vitest';
import { avatarCalls, clearMocks, openAvatar } from '@/test/open-avatar';
import { useAgentsStore } from './use-agents-store';
import { useAppStore } from './use-app-store';
import { useAvatarStore } from './use-avatar-store';

afterEach(() => clearMocks());

const state = () => useAvatarStore.getState();

describe('the avatar store', () => {
  it('opens the Avatar view on an asset and reads it', async () => {
    await openAvatar();
    state().open('frostveil', 'elsheria');
    expect(useAppStore.getState().view).toBe('avatar');
    await state().load();
    expect(state().view?.name).toBe('Elsheria');
    expect(state().view?.needsNormalize).toBe(true);
    expect(state().draft?.meta.name).toBe('Elsheria');
  });

  it('saves the meta, normalises, waits for a yes, then writes', async () => {
    const calls = await openAvatar();
    state().editMeta({ authors: ['Ana'] });
    expect(state().dirty).toBe(true);
    await state().build();
    expect(calls).toContain('avatar_save');
    expect(state().dirty).toBe(false);
    expect(state().view?.normalized?.state).toBe('pending');
    expect(state().view?.export).toBeNull();
    await state().decide(true);
    expect(state().view?.normalized?.state).toBe('approved');
    expect(state().view?.export?.path).toMatch(/exports\/elsheria\.vrm$/);
  });

  it("settles an agent's waiting request with a no", async () => {
    await openAvatar('pending');
    expect(useAgentsStore.getState().status?.approvals.map(a => a.kind)).toContain('rest-pose');
    await state().decide(false);
    expect(state().view?.normalized?.state).toBe('denied');
    expect(useAgentsStore.getState().status?.approvals.map(a => a.kind)).not.toContain('rest-pose');
  });

  it('reloads only the avatar an agent changed', async () => {
    const calls = await openAvatar();
    const before = calls.filter(c => c === 'avatar_get').length;
    state().onChanged('frostveil', 'veil-gate');
    state().onChanged('frostveil', 'elsheria');
    await Promise.resolve();
    expect(calls.filter(c => c === 'avatar_get').length).toBe(before + 1);
  });
});

const lastOf = (cmd: string) => avatarCalls.findLast(c => c.cmd === cmd);

describe('the avatar store versions', () => {
  it('opens on a version and goes back to the newest when another asset is picked', async () => {
    await openAvatar('older');
    state().open('frostveil', 'elsheria', 2);
    expect(state().version).toBe(2);
    state().select('clay-figure');
    expect(state().version).toBeNull();
  });

  it('passes the selected version to get, build and decide', async () => {
    await openAvatar('older');
    state().open('frostveil', 'elsheria', 2);
    await expect.poll(() => state().view?.version).toBe(2);
    expect(lastOf('avatar_get')?.args.version).toBe(2);
    await state().editMeta({ authors: ['The art team'] });
    await state().build();
    expect(lastOf('avatar_build')?.args.version).toBe(2);
    await state().decide(false);
    expect(lastOf('avatar_decide')?.args.version).toBe(2);
  });

  it('reloads the open avatar on its selected version when an agent changes it', async () => {
    await openAvatar('older');
    state().open('frostveil', 'elsheria', 2);
    await expect.poll(() => state().view?.version).toBe(2);
    state().onChanged('frostveil', 'elsheria');
    await expect.poll(() => avatarCalls.filter(c => c.cmd === 'avatar_get').length).toBe(3);
    expect(lastOf('avatar_get')?.args.version).toBe(2);
  });
});
