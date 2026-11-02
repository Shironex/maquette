import { afterEach, describe, expect, it, vi } from 'vitest';
import type { DecisionRecord } from '@/ipc/client';
import { decidedBy, decisionText, decisionTitle } from './decisions';

const AT = new Date(2026, 10, 1, 14, 2, 31).getTime();

const record = (patch: Partial<DecisionRecord> = {}): DecisionRecord => ({
  v: 1,
  atMs: AT,
  approval: 'approval-3',
  kind: 'send-to-3d',
  decision: 'approved',
  by: {
    source: 'floating-card',
    input: 'pointer',
    trusted: true,
    pageFocused: true,
    pageVisible: true,
    windowFocused: false,
    windowVisible: true,
    windowMinimized: false,
  },
  project: 'frostveil',
  jobs: ['veil-gate-3'],
  provider: 'Tripo',
  asset: 'Veil gate',
  estimate: 30,
  requestedAtMs: AT - 22_000,
  ...patch,
});

afterEach(() => vi.restoreAllMocks());

describe('who answered an approval', () => {
  it('reads a click as a pointer press and Enter or Space as the keyboard', () => {
    vi.spyOn(document, 'hasFocus').mockReturnValue(false);
    const click = decidedBy('agents-panel', { detail: 1, isTrusted: true });
    expect(click).toEqual({
      source: 'agents-panel',
      input: 'pointer',
      trusted: true,
      pageFocused: false,
      pageVisible: true,
    });
    expect(decidedBy('ship-view', { detail: 0, isTrusted: false })).toMatchObject({
      input: 'keyboard',
      trusted: false,
    });
    expect(decidedBy('floating-card')).toMatchObject({ input: null, trusted: null });
  });

  it('says in a few words who let a send go, when and from where', () => {
    expect(decisionText(record())).toBe('approved 14:02 in the floating card');
    expect(decisionText(record({ by: { source: 'agents-panel' } }))).toBe(
      'approved 14:02 in Settings > Agents'
    );
  });

  it('spells out the input and the window in the details', () => {
    const title = decisionTitle(record());
    expect(title).toContain('Approved');
    expect(title).toContain('22 s after the agent asked');
    expect(title).toContain('The page reported a press with the mouse or trackpad');
    expect(title).toContain('The system reported the Maquette window was not focused');
    expect(title).not.toMatch(/by a person|trusted/i);
    expect(decisionTitle(record({ by: { source: 'floating-card', input: 'keyboard' } }))).toContain(
      'a press with the keyboard'
    );
    expect(
      decisionTitle(record({ by: { source: 'floating-card', input: 'pointer', trusted: false } }))
    ).toContain('the page reported it as made by a script');
  });
});
