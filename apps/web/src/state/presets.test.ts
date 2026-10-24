import { describe, expect, it } from 'vitest';
import { CHARACTER_LOCK, pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { estimatedTransfer } from './pipeline';
import { estimateTransfer, modelLabel, presetRows, presetsToSave, splitSize } from './presets';

const view = pipelineView('moon-island', '/w', runHistory(0), []);

describe('presets', () => {
  it('labels an export of an asset version after the asset', () => {
    const clay = {
      ...view,
      models: [
        { model: 'clay_v1', assetName: 'clay', version: 1, rawBytes: 1, shippedBytes: null },
      ],
      def: { ...view.def, presets: [] },
    };
    expect(presetRows(clay).map(r => [r.model, r.label])).toEqual([['clay_v1', 'clay']]);
  });

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

  it('keeps one size by default and saves a split only when set', () => {
    const rows = presetRows(view);
    expect(rows.every(r => r.dataTextureSize === null)).toBe(true);
    const saved = presetsToSave(
      rows.map(r => (r.model === 'island' ? { ...r, dataTextureSize: 1024, custom: true } : r))
    );
    expect(saved.find(p => p.model === 'island')!.dataTextureSize).toBe(1024);
    expect(saved.find(p => p.model === 'sakura')!.dataTextureSize).toBeNull();
    expect(splitSize({ textureSize: 2048, dataTextureSize: 1024 })).toBe(1024);
    expect(splitSize({ textureSize: 1024, dataTextureSize: 2048 })).toBeNull();
    expect(splitSize({ textureSize: 1024, dataTextureSize: null })).toBeNull();
  });

  it('estimates a smaller transfer when data maps shrink, and none when nothing changed', () => {
    const rows = presetRows(view);
    expect(estimateTransfer(view, rows)).toBeCloseTo(estimatedTransfer(view));
    const split = rows.map(r =>
      r.model === 'island' ? { ...r, dataTextureSize: 512, custom: true } : r
    );
    const island = view.models.find(m => m.model === 'island')!;
    const base = island.shippedBytes ?? island.rawBytes ?? 0;
    const estimate = estimateTransfer(view, split);
    expect(estimate).toBeLessThan(estimatedTransfer(view));
    expect(estimatedTransfer(view) - estimate).toBeLessThan(base);
    // Undoing the split brings it back.
    expect(estimateTransfer(view, rows)).toBeGreaterThan(estimate);
  });
});
