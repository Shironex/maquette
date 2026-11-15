import { describe, expect, it } from 'vitest';
import type { LibraryRecipeSummary } from '@/ipc/client';
import { libraryLine } from './recipe-library';

const summary = (over: Partial<LibraryRecipeSummary> = {}): LibraryRecipeSummary => ({
  id: 'reach',
  uid: 'u1',
  name: 'Reach',
  version: 'v',
  latest: 3,
  revisions: [],
  base: 'Standing Idle',
  frames: 120,
  joined: [],
  length: 120,
  mixamo: false,
  layers: 3,
  blanks: 0,
  updatedAt: '2026-11-05T10:00:00.000Z',
  problem: null,
  ...over,
});

describe('a library recipe in one line', () => {
  it('names the base clip, frames, changes and revision', () => {
    expect(libraryLine(summary())).toBe('Standing Idle · 120 f · 3 changes · r3');
  });

  it('names the joined clips and the length they run together, as a starter does', () => {
    const joined = summary({ base: 'Scythe Charge', frames: 45, joined: ['A', 'B'], length: 97 });
    expect(libraryLine(joined)).toBe('Scythe Charge + 2 clips · 97 f · 3 changes · r3');
  });
});
