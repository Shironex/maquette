import { act, cleanup, render, renderHook, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { events } from '@/ipc/generated/bindings';
import { emptyLook } from '@/state/materials';
import { useMaterialsStore } from '@/state/use-materials-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useQuitStore } from '@/state/use-quit-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { QuitDialog, useQuitGuard } from '.';

afterEach(() => {
  // Unmount first: the listeners are removed through the mock.
  cleanup();
  clearMocks();
  usePipelineStore.setState({ view: null, presetEdits: {} });
  useMaterialsStore.setState({ saved: {}, edits: {}, saving: false });
  useQuitStore.setState({ asking: false, applying: false, error: null });
});

async function record() {
  const handle = await openMockWorkspace();
  const calls: [string, unknown][] = [];
  mockIPC(
    (cmd, args) => {
      calls.push([cmd, args]);
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    },
    { shouldMockEvents: true }
  );
  return calls;
}

const edit = {
  'sakura-tree': {
    model: 'sakura-tree',
    label: 'Sakura tree',
    textureSize: 512,
    dataTextureSize: null,
    codec: 'etc1s' as const,
    mapCodecs: null,
    meshopt: 'low' as const,
    lock: null,
    custom: true,
  },
};

describe('the quit guard', () => {
  it('tells the shell when presets wait and when they do not', async () => {
    const calls = await record();
    renderHook(() => useQuitGuard());
    const sent = () => calls.filter(([c]) => c === 'quit_set_unapplied').map(([, a]) => a);
    await waitFor(() => expect(sent()).toEqual([{ unapplied: false }]));

    act(() => usePipelineStore.getState().setPresetEdits('moon-island', edit));
    await waitFor(() => expect(sent().at(-1)).toEqual({ unapplied: true }));

    act(() => usePipelineStore.getState().setPresetEdits('moon-island', {}));
    await waitFor(() => expect(sent().at(-1)).toEqual({ unapplied: false }));
  });

  it('counts unapplied material tweaks too', async () => {
    const calls = await record();
    renderHook(() => useQuitGuard());
    const sent = () => calls.filter(([c]) => c === 'quit_set_unapplied').map(([, a]) => a);
    await waitFor(() => expect(sent()).toEqual([{ unapplied: false }]));
    const look = { ...emptyLook(), all: { ...emptyLook().all, metalness: 0.2 } };
    act(() => useMaterialsStore.getState().setLook('moon-island', 'sakura', look));
    await waitFor(() => expect(sent().at(-1)).toEqual({ unapplied: true }));
    act(() => useMaterialsStore.getState().reset('moon-island', 'sakura'));
    await waitFor(() => expect(sent().at(-1)).toEqual({ unapplied: false }));
  });
});

describe('the quit event', () => {
  it('opens the question when the shell holds a close back', async () => {
    await record();
    renderHook(() => useQuitGuard());
    expect(useQuitStore.getState().asking).toBe(false);
    await act(async () => {
      // Listeners register asynchronously; emit once they are in.
      await new Promise(r => setTimeout(r, 20));
      await events.quitRequested.emit();
    });
    await waitFor(() => expect(useQuitStore.getState().asking).toBe(true));
  });
});

describe('the quit dialog', () => {
  it('names the project and offers apply, quit and cancel', async () => {
    const calls = await record();
    usePipelineStore.getState().setPresetEdits('moon-island', edit);
    render(<QuitDialog />);
    expect(screen.queryByRole('dialog')).toBeNull();
    act(() => useQuitStore.getState().ask());
    const dialog = await screen.findByRole('dialog', { name: 'Apply the compression presets?' });
    expect(within(dialog).getByText(/Not applied yet in Moon Island/)).toBeVisible();

    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(calls.map(([c]) => c)).not.toContain('quit_now');
  });

  it('closes after Quit without applying', async () => {
    const calls = await record();
    usePipelineStore.getState().setPresetEdits('moon-island', edit);
    render(<QuitDialog />);
    act(() => useQuitStore.getState().ask());
    await userEvent.click(await screen.findByRole('button', { name: 'Quit without applying' }));
    await waitFor(() => expect(calls.map(([c]) => c)).toContain('quit_now'));
    expect(calls.map(([c]) => c)).not.toContain('pipeline_save_presets');
  });
});
