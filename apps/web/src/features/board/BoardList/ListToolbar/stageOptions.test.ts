import { describe, expect, it } from 'vitest';
import { STAGE_OPTIONS } from './stageOptions';

describe('the stage filter', () => {
  it('offers every stage after All', () => {
    expect(STAGE_OPTIONS.map(o => o.value)).toEqual([
      'all',
      'concept',
      'refs',
      'tripo',
      'blender',
      'web',
      'ship',
    ]);
  });
});
