import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { WORKSPACE_IN_USE, openWorkspace, useWorkspaceLock } from './use-workspace-lock';

const snapshot = { root: '/ws' } as Awaited<ReturnType<typeof commands.workspaceOpen>>;
const inUse = {
  code: WORKSPACE_IN_USE,
  message:
    'This workspace is open on Studio PC. Close Maquette there first, or Syncthing may create conflict copies.',
};

describe('opening a workspace another computer has open', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useWorkspaceLock.setState({ question: null });
  });

  it('opens at once when nobody else has it', async () => {
    const open = vi.spyOn(commands, 'workspaceOpen').mockResolvedValue(snapshot);
    await expect(openWorkspace('/ws')).resolves.toBe(snapshot);
    expect(open).toHaveBeenCalledWith('/ws', false);
    expect(useWorkspaceLock.getState().question).toBeNull();
  });

  it('asks, then opens anyway', async () => {
    const open = vi
      .spyOn(commands, 'workspaceOpen')
      .mockRejectedValueOnce(inUse)
      .mockResolvedValueOnce(snapshot);
    const opened = openWorkspace('/ws');
    await vi.waitFor(() => expect(useWorkspaceLock.getState().question).not.toBeNull());
    expect(useWorkspaceLock.getState().question?.message).toBe(inUse.message);
    useWorkspaceLock.getState().answer(true);
    await expect(opened).resolves.toBe(snapshot);
    expect(open).toHaveBeenLastCalledWith('/ws', true);
  });

  it('opens nothing on Cancel', async () => {
    const open = vi.spyOn(commands, 'workspaceOpen').mockRejectedValueOnce(inUse);
    const opened = openWorkspace('/ws');
    await vi.waitFor(() => expect(useWorkspaceLock.getState().question).not.toBeNull());
    useWorkspaceLock.getState().answer(false);
    await expect(opened).resolves.toBeNull();
    expect(open).toHaveBeenCalledTimes(1);
  });

  it('passes other errors on', async () => {
    vi.spyOn(commands, 'workspaceOpen').mockRejectedValue({
      code: 'not-a-workspace',
      message: 'x',
    });
    await expect(openWorkspace('/ws')).rejects.toMatchObject({ code: 'not-a-workspace' });
  });
});
