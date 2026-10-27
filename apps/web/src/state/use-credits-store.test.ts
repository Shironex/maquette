import { afterEach, describe, expect, it } from 'vitest';
import { mockIPC } from '@tauri-apps/api/mocks';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useCreditsStore } from './use-credits-store';

afterEach(() => {
  clearMocks();
  useCreditsStore.setState({ open: false, ledger: null, balances: null, error: null });
});

async function recorded() {
  const handle = await openMockWorkspace();
  const calls: [string, Record<string, unknown>][] = [];
  mockIPC((cmd, args) => {
    const payload = (args ?? {}) as Record<string, unknown>;
    calls.push([cmd, payload]);
    return handle(cmd, payload);
  });
  return calls;
}

describe('the credits store', () => {
  it('reads the ledger and the balances when the sheet opens', async () => {
    await recorded();
    useCreditsStore.getState().openCredits('frostveil');
    await useCreditsStore.getState().load();
    await useCreditsStore.getState().checkBalances();
    const { open, filter, ledger, balances } = useCreditsStore.getState();
    expect(open).toBe(true);
    expect(filter.project).toBe('frostveil');
    expect(ledger!.spends.length).toBeGreaterThan(5);
    expect(balances!.map(b => [b.service, b.key, b.credits])).toEqual([
      ['tripo', true, 1240],
      ['meshy', false, null],
    ]);
  });

  it('saves only the spends the filter shows', async () => {
    const calls = await recorded();
    await useCreditsStore.getState().load();
    useCreditsStore.getState().setFilter({ service: 'meshy' });
    await useCreditsStore.getState().exportCsv();
    const [, args] = calls.find(([cmd]) => cmd === 'credits_save_csv')!;
    const lines = String(args.csv).trimEnd().split('\n');
    expect(lines.length).toBeGreaterThan(1);
    expect(lines.slice(1).every(l => l.includes(',meshy,'))).toBe(true);
    expect(String(args.name)).toMatch(/^maquette-credits-\d{4}-\d{2}-\d{2}\.csv$/);
  });
});
