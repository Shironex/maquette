import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import type { DownloadsScan, ProviderJob } from '@/ipc/client';
import { jobKey } from './providers';
import { useProvidersStore } from './use-providers-store';
import { clearMocks } from '@/test/mock-ipc';

const job = (project: string, patch: Partial<ProviderJob> = {}): ProviderJob =>
  ({
    id: 'desk-lamp-1',
    project,
    asset: 'desk-lamp',
    assetName: 'Desk lamp',
    provider: 'by-hand',
    service: 'tripo',
    state: 'waiting',
    ...patch,
  }) as ProviderJob;

/** Records each IPC call and answers with `reply`. */
function record(reply: (cmd: string, args: Record<string, unknown>) => unknown) {
  const calls: [string, Record<string, unknown>][] = [];
  mockIPC((cmd, args) => {
    const payload = (args ?? {}) as Record<string, unknown>;
    calls.push([cmd, payload]);
    return reply(cmd, payload);
  });
  return (cmd: string) => calls.filter(([c]) => c === cmd).map(([, a]) => a);
}

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], busy: [], panel: null, downloads: null });
});

describe('the sends of the open project', () => {
  it('names a send by its project and its id in every call', async () => {
    const mine = job('moon');
    useProvidersStore.setState({ project: 'moon', jobs: [mine] });
    const sent = record((cmd, args) =>
      cmd === 'provider_cancel' || cmd === 'provider_hand_back'
        ? job(String(args.project), { state: cmd === 'provider_cancel' ? 'cancelled' : 'inbox' })
        : null
    );
    const store = useProvidersStore.getState();
    await store.openHandoff(mine);
    await store.revealImage(mine, 'front.png');
    await store.handBack(mine, '/Downloads/lamp.glb');
    await store.cancel(mine);
    const key = { project: 'moon', id: 'desk-lamp-1' };
    expect(sent('provider_open_handoff')).toEqual([key]);
    expect(sent('provider_reveal_image')).toEqual([{ ...key, image: 'front.png' }]);
    expect(sent('provider_hand_back')).toEqual([{ ...key, file: '/Downloads/lamp.glb' }]);
    expect(sent('provider_cancel')).toEqual([key]);
    expect(useProvidersStore.getState().busy).toEqual([]);
  });

  it('keeps another project send of the same id out of this project list', () => {
    const mine = job('moon');
    useProvidersStore.setState({ project: 'moon', jobs: [mine] });
    const { onEvent } = useProvidersStore.getState();
    // The other project's first desk lamp send lands in the Inbox.
    onEvent({ type: 'job', job: job('bay', { state: 'inbox' }) });
    expect(useProvidersStore.getState().jobs).toEqual([mine]);
    onEvent({ type: 'job', job: job('moon', { state: 'inbox' }) });
    expect(useProvidersStore.getState().jobs.map(j => [j.project, j.state])).toEqual([
      ['moon', 'inbox'],
    ]);
  });

  it('marks a send busy under its project, not its id alone', () => {
    expect(jobKey(job('moon'))).not.toBe(jobKey(job('bay')));
    useProvidersStore.setState({ busy: [jobKey(job('bay'))] });
    expect(useProvidersStore.getState().busy.includes(jobKey(job('moon')))).toBe(false);
  });

  it('hands a chosen download back to the send of the project that was scanned', async () => {
    const mine = job('moon');
    const scan = { folder: '/Downloads', sinceMs: 0, files: [], waiting: [mine] } as DownloadsScan;
    useProvidersStore.setState({
      project: 'moon',
      jobs: [mine],
      downloads: {
        project: 'moon',
        scan,
        error: null,
        choices: { '/Downloads/lamp.glb': 'desk-lamp-1' },
        used: [],
      },
    });
    const sent = record((cmd, args) =>
      cmd === 'provider_hand_back' ? job(String(args.project), { state: 'inbox' }) : null
    );
    await useProvidersStore.getState().useDownload('/Downloads/lamp.glb');
    expect(sent('provider_hand_back')).toEqual([
      { project: 'moon', id: 'desk-lamp-1', file: '/Downloads/lamp.glb' },
    ]);
    expect(useProvidersStore.getState().downloads?.used).toEqual(['/Downloads/lamp.glb']);
  });
});
