import { describe, expect, it } from 'vitest';
import type { PlanLogLine } from '@/ipc/client';
import { freshAgentLine, runIds, runsOf, unseenCount, whenOf } from './plan-log';

const line = (id: string, at: string, by: string | null, run: string | null = null, more = {}) =>
  ({
    id,
    at,
    by,
    run,
    summary: `Change ${id}`,
    mark: 'add',
    undone: false,
    undoable: true,
    ...more,
  }) as PlanLogLine;

// Newest first, as the core answers.
const LINES = [
  line('c6', '2026-11-02T12:41:00Z', 'Research agent', 'r2'),
  line('c5', '2026-11-02T12:40:00Z', 'Research agent', 'r2', { undone: true, undoable: false }),
  line('c4', '2026-11-02T12:31:00Z', null),
  line('c3', '2026-11-02T12:29:00Z', null),
  line('c2', '2026-11-02T09:00:00Z', null),
  line('c1', '2026-11-01T16:20:00Z', 'Research agent', 'r1'),
];

describe('the change log in runs', () => {
  it('groups an agent run and a person changes close in time', () => {
    const runs = runsOf(LINES);
    expect(runs.map(r => r.lines.map(l => l.id))).toEqual([
      ['c6', 'c5'],
      ['c4', 'c3'],
      ['c2'],
      ['c1'],
    ]);
    expect(runs[0]).toMatchObject({ by: 'Research agent', undoable: ['c6'] });
  });

  it('filters to agents or mine', () => {
    expect(runsOf(LINES, 'agents').flatMap(r => r.lines.map(l => l.id))).toEqual([
      'c6',
      'c5',
      'c1',
    ]);
    expect(runsOf(LINES, 'mine').map(r => r.lines.length)).toEqual([2, 1]);
  });

  it('finds the agent change worth a toast and the run it belongs to', () => {
    expect(freshAgentLine(LINES, '2026-11-02T12:00:00Z')?.id).toBe('c6');
    expect(freshAgentLine(LINES, '2026-11-02T12:45:00Z')).toBeNull();
    const undo = [
      line('c7', '2026-11-02T12:50:00Z', 'Research agent', null, { mark: 'undo' }),
      ...LINES,
    ];
    expect(freshAgentLine(undo, '2026-11-02T12:00:00Z')?.id).toBe('c6');
    expect(runIds(LINES, LINES[0]!)).toEqual(['c6']);
    expect(unseenCount(LINES, '2026-11-02T12:00:00Z')).toBe(2);
  });

  it('says when in the reader time', () => {
    const now = new Date(2026, 10, 2, 13, 0);
    expect(whenOf(new Date(2026, 10, 2, 9, 5).toISOString(), now)).toBe('09:05');
    expect(whenOf(new Date(2026, 10, 1, 9, 5).toISOString(), now)).toBe('Yesterday');
    expect(whenOf(new Date(2026, 9, 30, 9, 5).toISOString(), now)).toBe('Oct 30');
    expect(whenOf('not a time', now)).toBe('');
  });
});
