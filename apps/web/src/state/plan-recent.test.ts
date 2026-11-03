import { describe, expect, it } from 'vitest';
import type { PlanSummary } from '@/ipc/client';
import { newerOf, recentCanvases, renamed, seenFrom, seenKey, seeing } from './plan-recent';

const plan = (id: string, edited: string, more: Partial<PlanSummary> = {}): PlanSummary => ({
  id,
  file: `plans/${id}.plan.json`,
  name: id.replace(/-/g, ' '),
  preset: 'brainstorm',
  nodes: 0,
  edges: 0,
  images: 0,
  links: 0,
  assets: 0,
  stops: 0,
  camera: false,
  edited,
  problem: null,
  ...more,
});

const at = (iso: string) => Date.parse(iso);

describe('recent canvases on the rail', () => {
  const list = [
    plan('combat-ideas', '2026-11-01T10:00:00Z'),
    plan('boss-arena', '2026-10-28T10:00:00Z'),
    plan('opening-story', '2026-10-20T10:00:00Z'),
    plan('camera-plan', '2026-10-30T10:00:00Z', { camera: true }),
    plan('broken', '2026-11-02T10:00:00Z', { problem: 'Not JSON' }),
  ];

  it('lists the three opened or edited last, newest first, leaving out one that does not read', () => {
    const seen = { 'opening-story': at('2026-11-02T09:00:00Z') };
    expect(recentCanvases(list, seen).map(c => c.id)).toEqual([
      'opening-story',
      'combat-ideas',
      'camera-plan',
    ]);
  });

  it('marks a canvas edited since the person last had it open, never one never opened', () => {
    const seen = {
      'combat-ideas': at('2026-10-31T10:00:00Z'),
      'camera-plan': at('2026-10-30T11:00:00Z'),
    };
    const shown = recentCanvases(list, seen);
    expect(shown.find(c => c.id === 'combat-ideas')).toMatchObject({ changed: true });
    expect(shown.find(c => c.id === 'camera-plan')).toMatchObject({ changed: false, camera: true });
    expect(shown.find(c => c.id === 'boss-arena')).toMatchObject({ changed: false });
  });

  it('remembers seen times per workspace and project, dropping canvases gone from the list', () => {
    expect(seenKey('/w/one', 'frostveil')).not.toBe(seenKey('/w/two', 'frostveil'));
    expect(seenKey('/w/one', 'frostveil')).not.toBe(seenKey('/w/one', 'moon-island'));
    const next = seeing({ gone: 5, 'boss-arena': 9 }, ['combat-ideas', 'boss-arena'], 7, list);
    expect(next).toEqual({ 'combat-ideas': 7, 'boss-arena': 9 });
    expect(seenFrom({ a: 1, b: 'x', c: Number.NaN })).toEqual({ a: 1 });
    expect(seenFrom(null)).toEqual({});
    expect(seenFrom([1, 2])).toEqual({});
  });

  it('finds the canvases renamed between two reads', () => {
    const after = list.map(p => (p.id === 'boss-arena' ? { ...p, name: 'Warden arena' } : p));
    expect(renamed(list, after)).toEqual(['boss-arena']);
    expect(renamed([], after)).toEqual([]);
  });

  it('keeps a canvas a newer read put on the list when an older list read answers', () => {
    const from = new Map<string, number>([['boss-arena', 5]]);
    const shown = list.map(p => (p.id === 'boss-arena' ? { ...p, name: 'Warden arena' } : p));
    // A list read that started as read 3 answers after read 5 patched the arena.
    const merged = newerOf(shown, list, from, 3);
    expect(merged.find(p => p.id === 'boss-arena')?.name).toBe('Warden arena');
    expect(from.get('combat-ideas')).toBe(3);
    // A list read started after the patch takes the arena as listed.
    expect(newerOf(merged, list, from, 6).find(p => p.id === 'boss-arena')?.name).toBe(
      list.find(p => p.id === 'boss-arena')?.name
    );
    expect(from.get('boss-arena')).toBe(6);
    // A canvas missing from the read goes.
    expect(newerOf(merged, list.slice(1), from, 7)).toHaveLength(list.length - 1);
  });
});
