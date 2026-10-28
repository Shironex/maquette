import { describe, expect, it } from 'vitest';
import type { ImageApiStatus } from '@/ipc/client';
import { backendFor, formatUsd, imageSettings, roundCost, roundLine } from './image-api';

const status = (spent: number, pending = 0): ImageApiStatus => ({
  settings: { backends: { concepts: 'openai-images' }, quality: 'low', monthlyCapUsd: 10 },
  key: { saved: true, tail: 'k7Qm' },
  monthSpentUsd: spent,
  pendingUsd: pending,
  model: 'gpt-image-2.5-sunburst',
  estimates: { low: 0.011, medium: 0.058, high: 0.216 },
  pricesChecked: '2026-10-28',
  pricesUrl: 'https://developers.openai.com/api/docs/pricing',
});

describe('image settings', () => {
  it('fill in Codex, low quality and a $10 cap', () => {
    expect(imageSettings(undefined)).toEqual({
      backends: {
        concepts: 'codex',
        objectRefs: 'codex',
        otherViews: 'codex',
        cleanBackground: 'codex',
      },
      quality: 'low',
      monthlyCapUsd: 10,
    });
    expect(backendFor({ backends: { otherViews: 'openai-images' } }, 'otherViews')).toBe(
      'openai-images'
    );
    expect(backendFor({ backends: { otherViews: 'openai-images' } }, 'concepts')).toBe('codex');
  });

  it('format dollars in cents, and whole dollars at ten and above', () => {
    expect(formatUsd(0)).toBe('$0.00');
    expect(formatUsd(0.004)).toBe('<$0.01');
    expect(formatUsd(3.42)).toBe('$3.42');
    expect(formatUsd(10)).toBe('$10');
  });
});

describe('a round on the image api', () => {
  it('costs its images at the chosen quality', () => {
    const cost = roundCost(status(3.42), 4);
    expect(cost.usd).toBeCloseTo(0.044);
    expect(cost.over).toBe(false);
    expect(roundLine(status(3.42), 4)).toBe(
      'Runs on OpenAI Images (low), about $0.04 for 4. $3.42 of $10 spent this month.'
    );
  });

  it('is over the cap when spent, queued and new would pass it', () => {
    expect(roundCost(status(9.96), 4).over).toBe(true);
    expect(roundCost(status(9.9, 0.07), 4).over).toBe(true);
    expect(roundLine(status(9.96), 4)).toMatch(/would pass the monthly cap/);
  });
});
