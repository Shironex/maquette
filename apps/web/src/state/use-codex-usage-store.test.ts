import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createBackend } from '@/ipc/mock/backend';
import { POLL_MS, useCodexUsageStore } from './use-codex-usage-store';

const initial = useCodexUsageStore.getState();

function backend(codex: string, lab?: string) {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, codex, lab });
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>));
  return handle;
}

afterEach(() => {
  clearMocks();
  vi.useRealTimers();
  useCodexUsageStore.setState(initial, true);
});

describe('the Codex usage store', () => {
  it('reads now and then every minute while polled', async () => {
    const handle = backend('low');
    const calls = vi.fn();
    mockIPC((cmd, args) => {
      if (cmd === 'codex_usage_read') calls();
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    vi.useFakeTimers();
    const stop = useCodexUsageStore.getState().poll();
    await vi.waitFor(() =>
      expect(useCodexUsageStore.getState().usage?.primary?.usedPercent).toBe(86)
    );
    await vi.advanceTimersByTimeAsync(POLL_MS);
    expect(calls).toHaveBeenCalledTimes(2);
    stop();
    await vi.advanceTimersByTimeAsync(POLL_MS * 2);
    expect(calls).toHaveBeenCalledTimes(2);
  });

  it('takes a read the shell made', () => {
    backend('ok');
    const usage = { ...initial.usage, known: false } as never;
    useCodexUsageStore.getState().apply(usage);
    expect(useCodexUsageStore.getState().usage).toBe(usage);
  });

  it('keeps the held jobs waiting while the limit still holds', async () => {
    backend('limit', 'usage');
    const lifted = await useCodexUsageStore.getState().lift();
    expect(lifted?.lifted).toBe(false);
    expect(lifted?.jobs).toHaveLength(3);
    expect(lifted?.message).toMatch(/^The usage limit still holds until about \d\d:\d\d\./);
    expect(useCodexUsageStore.getState().usage?.limitReached).toBe('rate_limit_reached');
  });

  it('runs the held jobs again once the read says the limit lifted', async () => {
    backend('lifted', 'usage');
    const lifted = await useCodexUsageStore.getState().lift();
    expect(lifted?.lifted).toBe(true);
    expect(lifted?.message).toBe('The usage limit has lifted. 3 jobs run again now.');
    expect(lifted?.jobs.every(j => !j.usageLimit)).toBe(true);
    expect(useCodexUsageStore.getState().usage?.ordinaryUsageAllowed).toBe(true);
  });
});
