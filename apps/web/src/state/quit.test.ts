import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { emptyLook } from './materials';
import { useMaterialsStore } from './use-materials-store';
import { usePipelineStore } from './use-pipeline-store';
import { quitTitle, unappliedProjects, useQuitStore } from './use-quit-store';
import type { PresetEdits } from './presets';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null, presetEdits: {} });
  useMaterialsStore.setState({ saved: {}, edits: {}, saving: false });
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
  await usePipelineStore.getState().load('frostveil');
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
      mapCodecs: null,
      meshopt: 'low',
      lock: null,
      custom: true,
    },
  };
}

describe('quitting with presets not applied', () => {
  it('saves every unapplied project, clears the edits, and only then closes', async () => {
    const { calls } = await setup();
    usePipelineStore.getState().setPresetEdits('frostveil', editFirstRow());
    useQuitStore.getState().ask();

    await useQuitStore.getState().applyAndQuit();
    expect(calls).toContain('pipeline_save_presets');
    expect(calls.indexOf('pipeline_save_presets')).toBeLessThan(calls.indexOf('quit_now'));
    expect(usePipelineStore.getState().presetEdits).toEqual({});
    expect(useQuitStore.getState().error).toBeNull();
  });

  it('stays open with the error when the save fails', async () => {
    const { calls } = await setup(cmd => cmd === 'pipeline_save_presets');
    usePipelineStore.getState().setPresetEdits('frostveil', editFirstRow());
    useQuitStore.getState().ask();

    await useQuitStore.getState().applyAndQuit();
    expect(calls).not.toContain('quit_now');
    expect(useQuitStore.getState().asking).toBe(true);
    expect(useQuitStore.getState().error).toMatch(/Frostveil.*The disk is full/);
    expect(Object.keys(usePipelineStore.getState().presetEdits)).toEqual(['frostveil']);
  });

  it('quits without saving, and Cancel only closes the question', async () => {
    const { calls } = await setup();
    usePipelineStore.getState().setPresetEdits('frostveil', editFirstRow());
    useQuitStore.getState().ask();
    useQuitStore.getState().cancel();
    expect(useQuitStore.getState().asking).toBe(false);
    expect(calls).not.toContain('quit_now');

    await useQuitStore.getState().quitWithoutApplying();
    expect(calls).toContain('quit_now');
    expect(calls).not.toContain('pipeline_save_presets');
  });
});

describe('quitting with material tweaks not applied', () => {
  const tint = { ...emptyLook(), all: { ...emptyLook().all, tint: '#6f7446' } };

  it('saves the tweaks after the presets, then closes', async () => {
    const { calls } = await setup();
    usePipelineStore.getState().setPresetEdits('frostveil', editFirstRow());
    useMaterialsStore.getState().setLook('frostveil', 'veil-gate', tint);
    useQuitStore.getState().ask();

    await useQuitStore.getState().applyAndQuit();
    const order = ['pipeline_save_presets', 'materials_save', 'quit_now'].map(c =>
      calls.indexOf(c)
    );
    expect(order).toEqual([...order].sort((a, b) => a - b));
    expect(order[0]).toBeGreaterThanOrEqual(0);
    expect(useMaterialsStore.getState().edits).toEqual({});
  });

  it('stays open when the tweaks do not save', async () => {
    const { calls } = await setup(cmd => cmd === 'materials_save');
    useMaterialsStore.getState().setLook('frostveil', 'veil-gate', tint);
    await useQuitStore.getState().applyAndQuit();
    expect(calls).not.toContain('quit_now');
    expect(useQuitStore.getState().error).toMatch(/Frostveil.*The disk is full/);
  });

  it('names each project once and says what waits', () => {
    expect(unappliedProjects({ a: 1, b: 1 }, { b: 1, c: 1 })).toEqual(['a', 'b', 'c']);
    expect(quitTitle(true, false)).toBe('Apply the compression presets?');
    expect(quitTitle(false, true)).toBe('Apply the material tweaks?');
    expect(quitTitle(true, true)).toBe('Apply the presets and material tweaks?');
    expect(quitTitle(false, false, 2)).toBe('Quit before accepting the Inbox fix-ups?');
    expect(quitTitle(false, true, 2)).toBe('Apply the material tweaks?');
  });
});
