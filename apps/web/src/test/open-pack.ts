/**
 * Export pack tests share this: the library at scale with a pack run in
 * the mock backend until it settles. Tests run without the shell, so no
 * pack changed event reaches the store; `follow` reads the record instead.
 */
import { waitFor } from '@testing-library/react';
import { expect } from 'vitest';
import { commands, type PackRecord, type PackState } from '@/ipc/client';
import { DEFAULT_SETTINGS } from '@/ipc/mock/pack-fixtures';
import { usePackStore } from '@/state/use-pack-store';
import { openMockLibrary } from './mock-ipc';

/** The ids of the first `n` village props, in the library's order. */
export async function villageProps(n: number): Promise<string[]> {
  const view = await commands.libraryGet();
  const village = view.collections.find(c => c.id === 'village-props')!;
  return village.entries.slice(0, n);
}

/** Waits until pack `id` reaches one of `states`, and answers it. */
export async function follow(id: string, states: PackState[]): Promise<PackRecord> {
  let record: PackRecord | null = null;
  await waitFor(async () => {
    record = await commands.packGet(id);
    expect(states).toContain(record.state);
  });
  usePackStore.getState().onChanged(record!);
  return record!;
}

/** Opens the library at scale and runs a pack of `n` village props until it waits or is done. */
export async function runPack(n: number, name = 'village-props'): Promise<PackRecord> {
  await openMockLibrary('scale');
  const entries = await villageProps(n);
  const started = await commands.packStart({
    source: { from: 'entries', entries },
    settings: { ...DEFAULT_SETTINGS, name },
    out: null,
    preset: 'plain-obj',
  });
  usePackStore.setState({ open: true, record: started });
  return follow(started.id, ['waiting', 'done']);
}

export function resetPacks() {
  usePackStore.setState({
    open: false,
    source: null,
    settings: null,
    presets: [],
    stopping: null,
    plan: null,
    error: null,
    record: null,
    recent: [],
    busy: false,
    out: null,
  });
}
