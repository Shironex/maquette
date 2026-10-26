import { describe, expect, it } from 'vitest';
import { stageLabels } from '@/state/game';
import { stageOptions } from './stageOptions';

describe('the stage filter', () => {
  it('offers every stage after All', () => {
    expect(stageOptions(stageLabels('website')).map(o => o.value)).toEqual([
      'all',
      'concept',
      'refs',
      'tripo',
      'blender',
      'web',
      'ship',
    ]);
  });

  it('names the Web stage Export in a game', () => {
    expect(stageOptions(stageLabels('game')).find(o => o.value === 'web')?.label).toBe('Export');
  });
});
