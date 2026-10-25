import { describe, expect, it } from 'vitest';
import { LIST_COLUMNS } from './columns';

describe('the list columns', () => {
  it('sorts by each key once', () => {
    const keys = LIST_COLUMNS.map(c => c.key);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys[0]).toBe('name');
  });
});
