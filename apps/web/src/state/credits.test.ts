import { describe, expect, it } from 'vitest';
import type { Spend } from '@/ipc/client';
import {
  DAY_MS,
  balanceView,
  creditTotals,
  csvName,
  estimateText,
  filterSpends,
  formatCredits,
  ledgerProjects,
  shareText,
  spendsCsv,
} from './credits';

const NOW = 1_800_000_000_000;

const spend = (patch: Partial<Spend>): Spend => ({
  project: 'moon-island',
  job: 'sakura-1',
  asset: 'sakura',
  assetName: 'Sakura',
  service: 'tripo',
  task: 'task-1',
  summary: 'Tripo H3.1, 1 image',
  sentAtMs: NOW - DAY_MS,
  settledAtMs: NOW - DAY_MS + 60_000,
  credits: 30,
  estimated: false,
  outcome: 'succeeded',
  redoOf: null,
  viaAgent: false,
  ...patch,
});

describe('credit numbers', () => {
  it('formats whole and part credits', () => {
    expect(formatCredits(1240)).toBe('1,240');
    expect(formatCredits(12.5)).toBe('12.5');
    expect(formatCredits(null)).toBe('0');
    expect(estimateText(30)).toBe('About 30 credits');
    expect(estimateText(1)).toBe('About 1 credit');
    expect(estimateText(null)).toBeNull();
    expect(shareText(0.184)).toBe('18%');
  });
});

describe('the ledger view', () => {
  const spends = [
    spend({ job: 'sakura-2', redoOf: 'sakura-1', credits: 30 }),
    spend({
      job: 'spire-1',
      service: 'meshy',
      project: 'rooftop',
      sentAtMs: NOW - 20 * DAY_MS,
      credits: 20,
    }),
    spend({ job: 'old-1', sentAtMs: NOW - 90 * DAY_MS, credits: 50, estimated: true }),
    spend({ job: 'lamp-1', outcome: 'refunded', credits: 0 }),
  ];

  it('filters by project and provider', () => {
    expect(filterSpends(spends, { project: 'rooftop', service: null }).map(s => s.job)).toEqual([
      'spire-1',
    ]);
    expect(filterSpends(spends, { project: null, service: 'tripo' })).toHaveLength(3);
    expect(ledgerProjects(spends)).toEqual(['moon-island', 'rooftop']);
  });

  it('totals the last 7 and 30 days and the share that went on redos', () => {
    const totals = creditTotals(spends, NOW);
    expect(totals).toMatchObject({
      week: 30,
      month: 50,
      total: 100,
      sends: 4,
      redos: 1,
      estimated: 1,
    });
    expect(totals.redoShare).toBeCloseTo(0.3);
    expect(creditTotals([], NOW).redoShare).toBe(0);
  });

  it('writes CSV oldest first, quoting what needs it', () => {
    const csv = spendsCsv([
      spend({ summary: 'Meshy 6, "4K", PBR', job: 'b-1', sentAtMs: NOW }),
      spend({ job: 'a-1', sentAtMs: NOW - DAY_MS, redoOf: 'a-0', estimated: true }),
    ]);
    const [header, first, second] = csv.trimEnd().split('\n');
    expect(header).toBe(
      'sent_at,project,asset,job,provider,task,settings,credits,estimated,outcome,redo_of,via_agent'
    );
    expect(first).toContain(',a-1,tripo,task-1,');
    expect(first).toMatch(/,30,true,succeeded,a-0,false$/);
    expect(second).toContain('"Meshy 6, ""4K"", PBR"');
    expect(csvName(NOW)).toBe('maquette-credits-2027-01-15.csv');
  });

  it('reads a balance, a missing key and a failed ask', () => {
    expect(balanceView({ key: true, credits: 1240, held: 30, error: null }, false)).toEqual({
      value: '1,240',
      note: '30 held by running tasks',
      tone: 'ok',
    });
    expect(balanceView({ key: true, credits: 380, held: null, error: null }, false).note).toBe(
      'credits left'
    );
    expect(balanceView({ key: false, credits: null, held: null, error: null }, false).value).toBe(
      'No key'
    );
    expect(
      balanceView({ key: true, credits: null, held: null, error: 'Tripo refused the key.' }, false)
    ).toMatchObject({ value: 'Unknown', tone: 'error' });
    expect(balanceView(undefined, true).note).toBe('Asking');
  });
});
