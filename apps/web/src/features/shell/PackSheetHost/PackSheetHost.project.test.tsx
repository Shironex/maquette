import { act, render, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks } from '@/test/mock-ipc';
import { resetPacks, runPack } from '@/test/open-pack';
import { PackSheetHost } from '.';

vi.mock('@/features/library/PackSheet', () => ({ PackSheet: () => null }));

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  resetPacks();
});

describe('the pack sheet and the current project', () => {
  /** Another project's pack, waiting for an answer, with the host mounted. */
  async function othersPack() {
    const record = await runPack(3, 'others-pack');
    const other = { ...record, source: { from: 'project' as const, project: 'someone-else' } };
    vi.spyOn(commands, 'packGet').mockResolvedValue(other);
    render(<PackSheetHost />);
    return other;
  }

  it('stays open on the recent pack of another project', async () => {
    const pack = await othersPack();
    await act(() => usePackStore.getState().showRecord(pack.id));
    await waitFor(() => expect(usePackStore.getState().record?.id).toBe(pack.id));
    expect(usePackStore.getState().open).toBe(true);
  });

  it('stays open on resuming the pack of another project', async () => {
    const pack = await othersPack();
    vi.spyOn(commands, 'packResume').mockResolvedValue(pack);
    await act(() => usePackStore.getState().resume(pack.id));
    await waitFor(() => expect(usePackStore.getState().record?.id).toBe(pack.id));
    expect(usePackStore.getState().open).toBe(true);
  });

  it('closes when another project opens, and the pack can still be answered after', async () => {
    const pack = await runPack(3, 'left-behind');
    render(<PackSheetHost />);
    expect(usePackStore.getState().open).toBe(true);
    const workspace = useAppStore.getState().workspace!;
    act(() =>
      useAppStore.setState({
        workspace: { ...workspace, project: { ...workspace.project!, id: 'other' } },
      })
    );
    await waitFor(() => expect(usePackStore.getState().open).toBe(false));
    await act(() => usePackStore.getState().showRecord(pack.id));
    expect(usePackStore.getState().open).toBe(true);
    expect(usePackStore.getState().record?.id).toBe(pack.id);
    const decide = vi.spyOn(commands, 'packDecide').mockResolvedValue(pack);
    await act(() => usePackStore.getState().decide({} as never));
    expect(decide).toHaveBeenCalledWith(pack.id, {});
  });
});
