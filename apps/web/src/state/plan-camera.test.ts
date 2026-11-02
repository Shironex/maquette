import { describe, expect, it } from 'vitest';
import type { LayoutStop, PlanNode } from '@/ipc/client';
import {
  adopted,
  countsLine,
  discarded,
  fieldsLine,
  healed,
  leadEdge,
  moved,
  stopId,
  withNewStop,
} from './plan-camera';
import { flowNodeOf, type FlowEdge, type FlowNode } from './plans';

const stop = (id: string, fov = 35): LayoutStop => ({
  id,
  name: id[0]!.toUpperCase() + id.slice(1),
  position: [1, 1.6, 6],
  target: [0, 1, 0],
  fov,
  models: null,
});

const node = (id: string, s: LayoutStop, x: number, by: string | null = null): FlowNode =>
  flowNodeOf({
    id,
    kind: 'stop',
    x,
    y: 0,
    width: null,
    title: '',
    text: '',
    image: null,
    url: null,
    asset: null,
    direction: null,
    by,
    stop: s,
  } satisfies PlanNode);

/** Porch, schedule, moon joined in that order. */
function tour() {
  const nodes = [
    node('a', stop('porch'), 0),
    node('b', stop('schedule'), 280),
    node('c', stop('moon'), 560),
  ];
  const edges = [leadEdge('a', 'b', 1), leadEdge('b', 'c', 2)];
  return { nodes, edges, main: ['a', 'b', 'c'] };
}

const order = (edges: FlowEdge[], head: string) => {
  const out = [head];
  for (
    let to = edges.find(e => e.source === head)?.target;
    to;
    to = edges.find(e => e.source === to)?.target
  )
    out.push(to);
  return out;
};

describe('the camera plan on the canvas', () => {
  it('joins the stops around one taken out, so a line never splits', () => {
    const { edges } = tour();
    const left = edges.filter(e => e.source !== 'b' && e.target !== 'b');
    const next = healed(edges, left, ['b'], 3);
    expect(order(next, 'a')).toEqual(['a', 'c']);
    expect(next.at(-1)?.data?.edge.kind).toBe('leads-to');
  });

  it('moves a stop earlier or later and lays the line out again', () => {
    const t = tour();
    const next = moved(t.nodes, t.edges, t.main, 'c', -1, 5)!;
    expect(order(next.edges, 'a')).toEqual(['a', 'c', 'b']);
    expect(next.nodes.find(n => n.id === 'c')?.position.x).toBe(280);
    expect(moved(t.nodes, t.edges, t.main, 'a', -1, 5)).toBeNull();
  });

  it('adds a stop at the end of the main line with a free id and name', () => {
    const t = tour();
    const next = withNewStop(t.nodes, t.edges, t.main, 9);
    const added = next.nodes.find(n => n.id === next.id)!;
    expect(added.data.node.stop?.name).toBe('Stop 4');
    expect(added.data.node.stop?.id).toBe('stop-4');
    expect(order(next.edges, 'a')).toEqual(['a', 'b', 'c', next.id]);
    expect(stopId('Porch', new Set(['porch']))).toBe('porch-2');
  });

  it('takes a proposal as the main line: its order and settings, the person keeping their nodes', () => {
    const t = tour();
    const proposal = [
      node('p1', stop('porch'), 0, 'Camera agent'),
      node('p2', stop('moon', 28), 280, 'Camera agent'),
      node('p3', stop('lantern'), 560, 'Camera agent'),
    ];
    const edges = [...t.edges, leadEdge('p1', 'p2', 3), leadEdge('p2', 'p3', 4)];
    const next = adopted([...t.nodes, ...proposal], edges, t.main, ['p1', 'p2', 'p3'], 10);
    const mine = next.nodes.filter(n => n.data.node.by === null);
    expect(mine.map(n => n.data.node.stop?.id).sort()).toEqual(['lantern', 'moon', 'porch']);
    expect(next.nodes.find(n => n.id === 'c')?.data.node.stop?.fov).toBe(28);
    expect(next.nodes.some(n => n.id === 'b')).toBe(false);
    expect(order(next.edges, 'a').slice(0, 2)).toEqual(['a', 'c']);
    const gone = discarded(next.nodes, next.edges, ['p1', 'p2', 'p3']);
    expect(gone.nodes.some(n => n.data.node.by)).toBe(false);
  });

  it('counts and lists changes as the core writes them', () => {
    expect(countsLine({ added: 1, moved: 3, removed: 0, changed: 2 })).toBe(
      '1 stop added · 3 moved · 0 removed · 2 changed'
    );
    const fields = [{ field: 'fov', before: '35', after: '28' }];
    expect(fieldsLine(fields, false)).toBe(', fov');
    expect(fieldsLine(fields, true)).toBe('; fov 35 to 28');
    expect(fieldsLine([], true)).toBe('');
  });
});
