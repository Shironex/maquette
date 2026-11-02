import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type ToolReport } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { useAppStore } from './use-app-store';

const report = (version: string) =>
  ({ id: 'blender', label: 'Blender', status: 'found', version }) as ToolReport;

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(r => {
    resolve = r;
  });
  return { promise, resolve };
}

const blender = () => useAppStore.getState().tools.reports.find(r => r.id === 'blender');
const initial = useAppStore.getState().tools;

beforeEach(() => {
  useAppStore.setState({ tools: { ...initial, reports: [report('old')] } });
});

afterEach(() => {
  useAppStore.setState({ tools: initial });
  vi.restoreAllMocks();
});

describe('a tool check against a locate', () => {
  it('drops a check that started before the locate, even when it ends after', async () => {
    const check = deferred<ToolReport[]>();
    vi.spyOn(commands, 'toolchainDetect').mockReturnValue(check.promise as never);
    vi.spyOn(commands, 'dialogPickFile').mockResolvedValue('/apps/blender' as never);
    vi.spyOn(commands, 'toolchainLocate').mockResolvedValue(report('located') as never);
    const checking = useAppStore.getState().detectTools();
    await useAppStore.getState().locateTool('blender');
    check.resolve([report('stale')]);
    await checking;
    expect(blender()?.version).toBe('located');
    expect(useAppStore.getState().tools.checking).toBe(false);
  });

  it('keeps the check when the dialog was cancelled', async () => {
    const check = deferred<ToolReport[]>();
    vi.spyOn(commands, 'toolchainDetect').mockReturnValue(check.promise as never);
    vi.spyOn(commands, 'dialogPickFile').mockResolvedValue(null as never);
    const checking = useAppStore.getState().detectTools();
    await useAppStore.getState().locateTool('blender');
    check.resolve([report('fresh')]);
    await checking;
    expect(blender()?.version).toBe('fresh');
  });
});

describe('the re-check when the window comes back', () => {
  it('waits while the Locate dialog is open', async () => {
    const dialog = deferred<string | null>();
    const detect = vi.spyOn(commands, 'toolchainDetect').mockResolvedValue([] as never);
    vi.spyOn(commands, 'dialogPickFile').mockReturnValue(dialog.promise as never);
    const locating = useAppStore.getState().locateTool('blender');
    await useAppStore.getState().recheckToolsOnFocus();
    expect(detect).not.toHaveBeenCalled();
    dialog.resolve(null);
    await locating;
    await useAppStore.getState().recheckToolsOnFocus();
    expect(detect).toHaveBeenCalledOnce();
  });

  it('runs at most once in 30 seconds, so a dialog that takes focus cannot loop it', async () => {
    const detect = vi.spyOn(commands, 'toolchainDetect').mockResolvedValue([] as never);
    const now = vi.spyOn(clock, 'nowMs').mockReturnValue(1_000_000);
    const { recheckToolsOnFocus } = useAppStore.getState();
    await recheckToolsOnFocus();
    now.mockReturnValue(1_029_999);
    await recheckToolsOnFocus();
    expect(detect).toHaveBeenCalledOnce();
    now.mockReturnValue(1_030_000);
    await recheckToolsOnFocus();
    expect(detect).toHaveBeenCalledTimes(2);
  });
});
