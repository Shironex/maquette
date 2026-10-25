import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { assetKey, usePrepareStore } from './use-prepare-store';

let calls: string[] = [];

beforeEach(async () => {
  const handle = await openMockWorkspace();
  calls = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    return handle(cmd, (args ?? {}) as Record<string, unknown>);
  });
  usePrepareStore.setState({
    root: '/ws',
    assets: {},
    failed: {},
    drafts: {},
    previews: {},
    suggestions: {},
    saving: null,
  });
});
afterEach(() => clearMocks());

const key = assetKey('moon-island', 'character');

describe('the prepare store', () => {
  it('loads an asset, saves its entry and shows it at once', async () => {
    await usePrepareStore.getState().load('moon-island', 'character');
    expect(usePrepareStore.getState().assets[key]?.measured).toEqual([1.36, 2, 0.84]);
    const saving = usePrepareStore
      .getState()
      .save('moon-island', 'character', { size: null, fixups: [{ op: 'origin-bottom' }] });
    expect(usePrepareStore.getState().assets[key]?.entry.fixups).toEqual([{ op: 'origin-bottom' }]);
    expect(await saving).toBe(true);
    expect(calls).toContain('prepare_save');
  });

  it('puts the saved entry back and says why when a save is refused', async () => {
    await usePrepareStore.getState().load('moon-island', 'character');
    const ok = await usePrepareStore
      .getState()
      .save('moon-island', 'character', { size: null, fixups: [{ op: 'decimate', triangles: 3 }] });
    expect(ok).toBe(false);
    expect(usePrepareStore.getState().assets[key]?.entry.fixups).toEqual([]);
    expect(useAppStore.getState().notice?.title).toBe('Real size and fix-ups not saved');
  });

  it('runs a preview once per plan and model', async () => {
    const plan = { size: null, fixups: [{ op: 'decimate' as const, triangles: 50_000 }] };
    const request = { project: null, asset: null, inbox: 'character.glb', plan };
    const report = await usePrepareStore.getState().preview(request);
    expect(report?.after.triangles).toBe(50_000);
    await usePrepareStore.getState().preview(request);
    expect(calls.filter(c => c === 'prepare_preview')).toHaveLength(1);
  });

  it('drops an empty draft', () => {
    usePrepareStore.getState().setDraft('a.glb', { size: null, fixups: [{ op: 'origin-bottom' }] });
    expect(Object.keys(usePrepareStore.getState().drafts)).toEqual(['a.glb']);
    usePrepareStore.getState().setDraft('a.glb', { size: null, fixups: [] });
    expect(usePrepareStore.getState().drafts).toEqual({});
  });
});
