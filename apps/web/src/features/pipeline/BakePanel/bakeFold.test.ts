import { describe, expect, it } from 'vitest';
import { bakeFold, settingsLine, skippedText } from './bakeFold';

describe('when the Baked light card folds', () => {
  it('folds when every model skips the bake', () => {
    expect(bakeFold({ skipped: ['a', 'b', 'c'] }, ['a', 'b'], 'scene')).toBe(
      'All 2 models skip the bake, so this step bakes nothing.'
    );
    expect(bakeFold({ skipped: ['a'] }, ['a'], null)).toBe(
      'The only model skips the bake, so this step bakes nothing.'
    );
  });

  it('folds for a hand-built site and says how many skip', () => {
    expect(bakeFold({ skipped: ['a'] }, ['a', 'b'], 'models')).toBe(
      'Ships to a hand-built site. 1 model skips the bake and keeps its own lighting.'
    );
  });

  it('stays open while some model bakes on a scene site, or with no models', () => {
    expect(bakeFold({ skipped: ['a'] }, ['a', 'b'], 'scene')).toBeNull();
    expect(bakeFold({ skipped: [] }, [], 'scene')).toBeNull();
  });

  it('puts the settings in one line', () => {
    expect(settingsLine({ size: 1024, samples: 32, denoise: false })).toBe(
      '1024 px · 32 samples · no denoise'
    );
    expect(skippedText(0)).toBe('Every model is baked.');
  });
});
