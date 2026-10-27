import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useProvidersStore } from './use-providers-store';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], panel: null, batch: null, sending: false });
});

/** The redo fixture, recording each command sent. */
async function open() {
  const handle = await openMockWorkspace('configured', undefined, undefined, 'redo');
  const calls: [string, Record<string, unknown>][] = [];
  mockIPC((cmd, args) => {
    const payload = (args ?? {}) as Record<string, unknown>;
    calls.push([cmd, payload]);
    return handle(cmd, payload);
  });
  await useProvidersStore.getState().load('frostveil');
  return (cmd: string) => calls.filter(([c]) => c === cmd).map(([, a]) => a);
}

describe('Redo from the Inbox', () => {
  it('opens the send panel prefilled from the earlier send', async () => {
    await open();
    await useProvidersStore.getState().openRedo('frostveil', 'veil-gate', 'veil-gate-2');
    const panel = useProvidersStore.getState().panel!;
    expect(panel.redo?.id).toBe('veil-gate-2');
    expect(panel.draft).toMatchObject({
      provider: 'meshy',
      service: 'meshy',
      variant: 'prop',
      images: ['refs/veil_gate_front.png'],
      prompt: 'Pale frosted stone, violet rune glow, soft painted look',
    });
    expect(panel.draft!.meshy.textureResolution).toBe('4k');
    // Nothing changed yet, so closing does not ask.
    expect(panel.initial).toEqual(panel.draft);
  });

  it('sends as a new version that names the send it redoes', async () => {
    const sent = await open();
    await useProvidersStore.getState().openRedo('frostveil', 'veil-gate', 'veil-gate-2');
    useProvidersStore.getState().setDraft({ prompt: '  Mossy bark  ' });
    await useProvidersStore.getState().send();
    const [args] = sent('provider_send');
    expect(args!.request).toMatchObject({
      asset: 'veil-gate',
      provider: 'meshy',
      prompt: 'Mossy bark',
      redoOf: 'veil-gate-2',
    });
    const redo = useProvidersStore.getState().jobs.find(j => j.redoOf === 'veil-gate-2');
    expect(redo).toMatchObject({ id: 'veil-gate-3', prompt: 'Mossy bark', estimate: 30 });
    // The earlier send, and its model in the inbox, stay as they were.
    expect(useProvidersStore.getState().jobs.find(j => j.id === 'veil-gate-2')?.state).toBe(
      'inbox'
    );
  });

  it('says so when the earlier send is gone', async () => {
    await open();
    await useProvidersStore.getState().openRedo('frostveil', 'veil-gate', 'veil-gate-9');
    expect(useProvidersStore.getState().panel?.error).toMatch(/no longer on record/);
  });

  it('opens a batch with each asset prefilled from its own send', async () => {
    const sent = await open();
    await useProvidersStore.getState().openBatchRedo('frostveil', [
      { asset: 'elsheria', job: 'elsheria-3' },
      { asset: 'veil-gate', job: 'veil-gate-2' },
    ]);
    const batch = useProvidersStore.getState().batch!;
    expect(batch.redo).toBe(true);
    expect(batch.draft).toMatchObject({ provider: 'tripo', service: 'tripo' });
    expect(batch.draft!.tripo.character.textureQuality).toBe('detailed');
    expect(batch.draft!.items.map(i => [i.asset, i.variant, i.redoOf])).toEqual([
      ['elsheria', 'character', 'elsheria-3'],
      ['veil-gate', 'prop', 'veil-gate-2'],
    ]);
    useProvidersStore.getState().setItemPrompt('veil-gate', 'Ink wash');
    expect(useProvidersStore.getState().batch!.draft!.items[1]!.prompt).toBe('Ink wash');
    // A paid batch asks first, then goes.
    await useProvidersStore.getState().sendBatch();
    await useProvidersStore.getState().sendBatch();
    const [args] = sent('provider_send_batch');
    expect((args!.request as { items: { redoOf: string }[] }).items.map(i => i.redoOf)).toEqual([
      'elsheria-3',
      'veil-gate-2',
    ]);
  });

  it('a plain Send panel opened before a Redo loads stays a plain send, and the other way round', async () => {
    await open();
    const store = useProvidersStore.getState();
    const redo = store.openRedo('frostveil', 'veil-gate', 'veil-gate-2');
    const plain = store.openPanel('frostveil', 'veil-gate');
    await Promise.all([redo, plain]);
    let panel = useProvidersStore.getState().panel!;
    expect(panel.redo ?? null).toBeNull();
    expect(panel.draft?.prompt ?? null).not.toBe(
      'Pale frosted stone, violet rune glow, soft painted look'
    );

    const plainFirst = store.openPanel('frostveil', 'veil-gate');
    const redoAfter = store.openRedo('frostveil', 'veil-gate', 'veil-gate-2');
    await Promise.all([plainFirst, redoAfter]);
    panel = useProvidersStore.getState().panel!;
    expect(panel.redo?.id).toBe('veil-gate-2');
    expect(panel.draft?.provider).toBe('meshy');
  });
});
