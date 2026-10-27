import { describe, expect, it } from 'vitest';
import type { PresetRow } from '@/state/presets';
import { modelResults, modelsLine, resultText, sortRows } from './modelResults';

const file = (model: string, raw: number | null, shipped: number | null) => ({
  model,
  assetName: null,
  version: null,
  rawBytes: raw,
  shippedBytes: shipped,
});

const row = (model: string, label: string) => ({ model, label }) as PresetRow;

describe('the model results beside the presets', () => {
  it('sums every model into one line', () => {
    expect(modelsLine({ models: [] })).toBe('No models yet');
    expect(modelsLine({ models: [file('a', null, null)] })).toBe('1 model, not exported yet');
    expect(modelsLine({ models: [file('a', 2e6, null)] })).toBe(
      '1 model · 2.0 MB raw, not compressed yet'
    );
    expect(modelsLine({ models: [file('a', 20e6, 1e6), file('b', 7.4e6, 1.3e6)] })).toBe(
      '2 models · 27.4 MB → 2.3 MB'
    );
  });

  it('says what one model ships at', () => {
    expect(resultText(undefined)).toBe('not exported');
    expect(resultText({ raw: 803e3, shipped: null })).toBe('803 KB raw');
    expect(resultText({ raw: 803e3, shipped: 60e3 })).toBe('60 KB');
  });

  it('sorts by name or by size, largest first and unknown last', () => {
    const rows = [row('b', 'Bench'), row('a', 'Arc floor'), row('c', 'Crate')];
    const results = modelResults({ models: [file('a', 1e6, 50e3), file('b', 3e6, 20e3)] });
    expect(sortRows(rows, results, 'name').map(r => r.model)).toEqual(['a', 'b', 'c']);
    expect(sortRows(rows, results, 'shipped').map(r => r.model)).toEqual(['a', 'b', 'c']);
    expect(sortRows(rows, results, 'raw').map(r => r.model)).toEqual(['b', 'a', 'c']);
  });
});
