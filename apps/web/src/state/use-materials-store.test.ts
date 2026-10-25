import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { commands, type ModelMaterials } from '@/ipc/client';
import { fail as mockFail } from '@/ipc/mock/backend-options';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { emptyLook } from './materials';
import { useAppStore } from './use-app-store';
import { useMaterialsStore } from './use-materials-store';

afterEach(() => {
  clearMocks();
  useMaterialsStore.setState({ root: null, saved: {}, edits: {}, failed: {}, saving: false });
});

const tinted = (tint: string): ModelMaterials => ({
  ...emptyLook(),
  all: { ...emptyLook().all, tint },
});

async function setup(fail?: (cmd: string) => boolean) {
  const handle = await openMockWorkspace();
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    if (fail?.(cmd)) mockFail('io', 'The disk is full.');
    return handle(cmd, (args ?? {}) as Record<string, unknown>);
  });
  await useMaterialsStore.getState().load('moon-island');
  return calls;
}

describe('the material tweaks store', () => {
  it('keeps edits per project until Apply saves them', async () => {
    const calls = await setup();
    const store = useMaterialsStore.getState();
    store.setLook('moon-island', 'sakura', tinted('#FFC0CB'));
    expect(Object.keys(useMaterialsStore.getState().edits)).toEqual(['moon-island']);

    expect(await useMaterialsStore.getState().apply('moon-island')).toBe(true);
    expect(calls).toContain('materials_save');
    expect(useMaterialsStore.getState().edits).toEqual({});
    expect(useMaterialsStore.getState().saved['moon-island']?.models.sakura).toEqual(
      tinted('#ffc0cb')
    );
    expect((await commands.materialsGet('moon-island')).models.sakura?.all.tint).toBe('#ffc0cb');
  });

  it('Reset drops the edit, and an edit back at the saved look is none', async () => {
    await setup();
    const store = useMaterialsStore.getState();
    store.setLook('moon-island', 'sakura', tinted('#ff0000'));
    store.reset('moon-island', 'sakura');
    expect(useMaterialsStore.getState().edits).toEqual({});
    store.setLook('moon-island', 'sakura', emptyLook());
    expect(useMaterialsStore.getState().edits).toEqual({});
  });

  it('drops an edit an agent already saved when it loads again', async () => {
    await setup();
    useMaterialsStore.getState().setLook('moon-island', 'sakura', tinted('#00ff00'));
    await commands.materialsSave('moon-island', { sakura: tinted('#00ff00') });
    await useMaterialsStore.getState().load('moon-island');
    expect(useMaterialsStore.getState().edits).toEqual({});
  });

  it('keeps the edits and says why when the save fails', async () => {
    await setup(cmd => cmd === 'materials_save');
    useMaterialsStore.getState().setLook('moon-island', 'sakura', tinted('#00ff00'));
    expect(await useMaterialsStore.getState().apply('moon-island')).toBe(false);
    expect(Object.keys(useMaterialsStore.getState().edits)).toEqual(['moon-island']);
    expect(useAppStore.getState().notice?.title).toBe('Material tweaks not saved');
    expect(await useMaterialsStore.getState().applyAll()).toMatch(/Moon Island: The disk is full/);
  });

  it('refuses a value out of range like the core does', async () => {
    await setup();
    const bad = { ...emptyLook(), all: { ...emptyLook().all, metalness: 3 } };
    await expect(commands.materialsSave('moon-island', { sakura: bad })).rejects.toMatchObject({
      message: expect.stringMatching(/metalness/),
    });
  });

  it('Apply sends only the edited looks, so an unloaded file keeps the other assets', async () => {
    await setup();
    await commands.materialsSave('moon-island', { hut: tinted('#123456') });
    // As if the load never finished or failed: nothing saved is known.
    useMaterialsStore.setState({ saved: {} });
    useMaterialsStore.getState().setLook('moon-island', 'sakura', tinted('#00ff00'));
    expect(await useMaterialsStore.getState().apply('moon-island')).toBe(true);
    const stored = await commands.materialsGet('moon-island');
    expect(stored.models.hut?.all.tint).toBe('#123456');
    expect(stored.models.sakura?.all.tint).toBe('#00ff00');
    expect(useMaterialsStore.getState().saved['moon-island']).toEqual(stored);
  });

  it('drops what is saved and the edits when another workspace opens', async () => {
    await setup();
    const store = useMaterialsStore.getState();
    store.setRoot('/work/one');
    await useMaterialsStore.getState().load('moon-island');
    useMaterialsStore.getState().setLook('moon-island', 'sakura', tinted('#00ff00'));
    useMaterialsStore.getState().setRoot('/work/one');
    expect(Object.keys(useMaterialsStore.getState().edits)).toEqual(['moon-island']);

    const late = useMaterialsStore.getState().load('moon-island');
    useMaterialsStore.getState().setRoot('/work/two');
    await late;
    expect(useMaterialsStore.getState().edits).toEqual({});
    expect(useMaterialsStore.getState().saved).toEqual({});
  });

  it('remembers why the saved tweaks did not load', async () => {
    await setup(cmd => cmd === 'materials_get');
    await useMaterialsStore.getState().load('moon-island');
    expect(useMaterialsStore.getState().failed['moon-island']).toBe('The disk is full.');
    expect(useMaterialsStore.getState().saved['moon-island']).toBeUndefined();
  });
});
