import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { usePipelineStore } from './pipeline-store';
import { useQuitStore } from './quit-store';
import type { PresetEdits } from './presets';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null, presetEdits: {} });
  useQuitStore.setState({ asking: false, applying: false, error: null });
});

/** Opens the mock workspace and records the commands the quit flow sends. */
async function setup(fail?: (cmd: string) => boolean) {
  const handle = await openMockWorkspace();
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    if (fail?.(cmd)) throw { code: 'io', message: 'The disk is full.' };
    return handle(cmd, (args ?? {}) as Record<string, unknown>);
  });
  await usePipelineStore.getState().load('moon-island');
  const view = usePipelineStore.getState().view!;
  return { calls, view };
}

/** One row changed from its saved size. */
function editFirstRow(): PresetEdits {
  const view = usePipelineStore.getState().view!;
  const row = view.models[0]!.model;
  return {
    [row]: {
      model: row,
      label: row,
      textureSize: 512,
      dataTextureSize: null,
      codec: 'etc1s',
      meshopt: 'low',
      lock: null,
      custom: true,
    },
  };
}

describe('quitting with presets not applied', () => {
  it('saves every unapplied project, clears the edits, and only then closes', async () => {
    const { calls } = await setup();
    usePipelineStore.getState().setPresetEdits('moon-island', editFirstRow());
    useQuitStore.getState().ask();

    await useQuitStore.getState().applyAndQuit();
    expect(calls).toContain('pipeline_save_presets');
    expect(calls.indexOf('pipeline_save_presets')).toBeLessThan(calls.indexOf('quit_now'));
    expect(usePipelineStore.getState().presetEdits).toEqual({});
    expect(useQuitStore.getState().error).toBeNull();
  });

  it('stays open with the error when the save fails', async () => {
    const { calls } = await setup(cmd => cmd === 'pipeline_save_presets');
    usePipelineStore.getState().setPresetEdits('moon-island', editFirstRow());
    useQuitStore.getState().ask();

    await useQuitStore.getState().applyAndQuit();
    expect(calls).not.toContain('quit_now');
    expect(useQuitStore.getState().asking).toBe(true);
    expect(useQuitStore.getState().error).toMatch(/Moon Island.*The disk is full/);
    expect(Object.keys(usePipelineStore.getState().presetEdits)).toEqual(['moon-island']);
  });

  it('quits without saving, and Cancel only closes the question', async () => {
    const { calls } = await setup();
    usePipelineStore.getState().setPresetEdits('moon-island', editFirstRow());
    useQuitStore.getState().ask();
    useQuitStore.getState().cancel();
    expect(useQuitStore.getState().asking).toBe(false);
    expect(calls).not.toContain('quit_now');

    await useQuitStore.getState().quitWithoutApplying();
    expect(calls).toContain('quit_now');
    expect(calls).not.toContain('pipeline_save_presets');
  });
});
