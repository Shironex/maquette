import { describe, expect, it } from 'vitest';
import { CHARACTER_LOCK, pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { estimatedTransfer } from './pipeline';
import { modelLabel, presetRows, presetsToSave } from './presets';

const view = pipelineView('moon-island', '/w', runHistory(0), []);

describe('presets', () => {
  it('labels model stems', () => {
    expect(modelLabel('sakura_large')).toBe('Sakura large');
    expect(modelLabel('shop-house')).toBe('Shop house');
  });

  it('lists every model with stored rows or the defaults', () => {
    const rows = presetRows(view);
    expect(rows.map(r => r.model)).toEqual([
      'character',
      'island',
      'sakura',
      'shop_house',
      'vending',
    ]);
    const character = rows.find(r => r.model === 'character')!;
    expect(character).toMatchObject({ meshopt: 'medium', lock: CHARACTER_LOCK, custom: true });
    expect(rows.find(r => r.model === 'island')).toMatchObject({
      textureSize: 2048,
      codec: 'etc1s',
      meshopt: 'high',
      custom: false,
    });
  });

  it('saves stored and edited rows only, keeping locks', () => {
    const rows = presetRows(view).map(r =>
      r.model === 'island' ? { ...r, textureSize: 1024, custom: true } : r
    );
    const saved = presetsToSave(rows);
    expect(saved.map(p => p.model)).toEqual(['character', 'island', 'sakura', 'vending']);
    expect(saved.find(p => p.model === 'island')!.textureSize).toBe(1024);
    expect(saved.find(p => p.model === 'character')!.lock).toBe(CHARACTER_LOCK);
    expect(Object.keys(saved[0]!)).not.toContain('custom');
  });

  it('adds up shipped bytes', () => {
    expect(estimatedTransfer(view)).toBeCloseTo(3.8e6);
  });
});
