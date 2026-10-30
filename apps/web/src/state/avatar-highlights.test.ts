import { describe, expect, it } from 'vitest';
import type { HighlightsReport } from '@/ipc/client';
import { HIGHLIGHTS, highlightsSummary } from './avatar-highlights';

const image = { image: 1, width: 2048, height: 2048, darkCoverage: 0.032, note: null };

describe('the highlights summary', () => {
  it('says how much was softened, kept, or why nothing changed', () => {
    const report: HighlightsReport = {
      mode: 'soften',
      strength: 0.75,
      images: [{ ...image, coverage: 0.0298, softened: true }],
      note: null,
    };
    expect(highlightsSummary(report)).toBe('3.0% softened');
    expect(highlightsSummary({ ...report, mode: 'keep', images: [] })).toBe('Kept as they are');
    expect(
      highlightsSummary({
        ...report,
        images: [{ ...image, coverage: 0, softened: false }],
        note: 'No strong baked highlights were found.',
      })
    ).toBe('No strong baked highlights were found.');
  });

  it('offers soften first, the default', () => {
    expect(HIGHLIGHTS.map(h => h.value)).toEqual(['soften', 'keep']);
  });
});
