import { describe, expect, it } from 'vitest';
import type { ConceptJob } from '@/ipc/client';
import { mockUsage } from '@/ipc/mock/create-codex-usage-handlers';
import { formatClock } from '@/lib/format';
import {
  heldBanner,
  limitActive,
  limitResetsAt,
  resetTime,
  roundWarning,
  usageDetail,
  usageLabel,
  usageTone,
  windowName,
} from './codex-usage';

const NOW = new Date(2026, 9, 28, 10, 0).getTime();
const HOUR = 3_600_000;

describe('Codex usage', () => {
  it('names each window and puts both in the status line', () => {
    const usage = mockUsage('ok', NOW);
    expect(usageLabel(usage)).toBe('Codex 16% (5h) · 3% (week)');
    expect(windowName({ usedPercent: 1, windowMins: 90, resetsAtMs: null })).toBe('90m');
    expect(windowName({ usedPercent: 1, windowMins: 2880, resetsAtMs: null })).toBe('2d');
    expect(usageLabel(mockUsage('unknown', NOW))).toBe('Codex usage unknown');
  });

  it('turns amber from 80% and red while the limit holds', () => {
    expect(usageTone(mockUsage('ok', NOW))).toBe('ok');
    expect(usageTone(mockUsage('low', NOW))).toBe('warn');
    expect(usageTone(mockUsage('limit', NOW))).toBe('limit');
    expect(usageTone(mockUsage('unknown', NOW))).toBe('ok');
    expect(usageTone(null)).toBe('ok');
  });

  it('knows when the limit holds and when it lifts', () => {
    const limit = mockUsage('limit', NOW);
    expect(limitActive(limit)).toBe(true);
    expect(limitResetsAt(limit)).toBe(limit.primary?.resetsAtMs);
    // A full week waits for the later reset.
    const week = {
      ...limit,
      secondary: { usedPercent: 100, windowMins: 10080, resetsAtMs: NOW + 50 * HOUR },
    };
    expect(limitResetsAt(week)).toBe(NOW + 50 * HOUR);
    expect(limitActive(mockUsage('low', NOW))).toBe(false);
    expect(limitResetsAt(mockUsage('low', NOW))).toBeNull();
    expect(limitActive(mockUsage('unknown', NOW))).toBe(false);
  });

  it('gives each reset in local time, with the day when it is not today', () => {
    expect(resetTime(NOW + HOUR, NOW)).toBe(formatClock(NOW + HOUR));
    expect(resetTime(NOW + 26 * HOUR, NOW)).toMatch(/^Thu \d\d:\d\d$/);
    const detail = usageDetail(mockUsage('limit', NOW), NOW);
    expect(detail).toMatch(/^Usage limit reached until about \d\d:\d\d\. 5h window: 100% used/);
    expect(detail).toContain('Weekly: 44% used, resets Fri');
    expect(usageDetail(mockUsage('unknown', NOW), NOW)).toContain('cannot report its usage');
  });

  it('warns before a large round when little is left, and before any while the limit holds', () => {
    const low = mockUsage('low', NOW);
    expect(roundWarning(low, 3, NOW)).toBeNull();
    expect(roundWarning(low, 8, NOW)).toBe(
      `Only 14% of Codex's 5h usage is left, which resets ${formatClock(NOW + 1.4 * HOUR)}. A round of 8 may stop at the usage limit and wait.`
    );
    expect(roundWarning(mockUsage('ok', NOW), 12, NOW)).toBeNull();
    expect(roundWarning(mockUsage('limit', NOW), 1, NOW)).toMatch(
      /^Codex's usage limit holds until about \d\d:\d\d\. These images wait in the queue until then\.$/
    );
    expect(roundWarning(mockUsage('unknown', NOW), 12, NOW)).toBeNull();
  });

  it("trusts the read's reset time over the jobs' own in the Lab banner", () => {
    const job = (patch: Partial<ConceptJob>) =>
      ({ state: 'rate-limited', usageLimit: true, retryAtMs: NOW + HOUR, ...patch }) as ConceptJob;
    expect(heldBanner([job({ usageLimit: false })], null, NOW)).toBeNull();
    const limit = mockUsage('limit', NOW);
    expect(heldBanner([job({}), job({})], limit, NOW)).toEqual({
      title: 'Codex usage limit reached',
      detail: `Codex says it lifts at about ${formatClock(limit.primary!.resetsAtMs!)}. 2 images wait and run again on their own then.`,
    });
    expect(heldBanner([job({})], mockUsage('unknown', NOW), NOW)?.detail).toBe(
      `1 image waits and runs again on their own at about ${formatClock(NOW + HOUR)}.`
    );
    expect(heldBanner([job({ retryAtMs: null })], null, NOW)?.detail).toBe(
      '1 image waits until it lifts.'
    );
  });
});
