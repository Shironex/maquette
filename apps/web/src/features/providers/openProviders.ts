/** Test support: the mock workspace with its 3D sends loaded, recording each IPC command. */
import { mockIPC } from '@tauri-apps/api/mocks';
import type { ProviderJob } from '@/ipc/client';
import { useProvidersStore } from '@/state/use-providers-store';
import { openMockWorkspace } from '@/test/mock-ipc';

export async function openProviders(handoff?: string, inbox?: string) {
  const handle = await openMockWorkspace('configured', undefined, handoff, inbox);
  const calls: [string, Record<string, unknown>][] = [];
  mockIPC((cmd, args) => {
    const payload = (args ?? {}) as Record<string, unknown>;
    calls.push([cmd, payload]);
    return handle(cmd, payload);
  });
  await useProvidersStore.getState().load('frostveil');
  const job = (id: string): ProviderJob => {
    const found = useProvidersStore.getState().jobs.find(j => j.id === id);
    if (!found) throw new Error(`No mock send ${id}`);
    return found;
  };
  const sent = (cmd: string) => calls.filter(([c]) => c === cmd).map(([, a]) => a);
  return { job, sent };
}
