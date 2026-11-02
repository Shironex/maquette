import { describe, expect, it } from 'vitest';
import type { PlanFile, PlanSummary } from '@/ipc/client';
import {
  byEdited,
  contentLine,
  coord,
  findPlans,
  flowEdgesOf,
  flowNodesOf,
  freeSpot,
  hostOf,
  LIMITS,
  newEdge,
  newId,
  newNode,
  nodeLabel,
  nodeTitle,
  planOf,
  samePlan,
  webAddress,
} from './plans';

const plan = (): PlanFile => ({
  version: 2,
  name: 'Combat ideas',
  preset: 'brainstorm',
  created: '2026-11-01T10:00:00.000Z',
  edited: '2026-11-01T10:00:00.000Z',
  nodes: [
    {
      id: 'n-1',
      kind: 'note',
      x: 10,
      y: 20,
      width: null,
      title: '',
      text: 'Frost veil slows time',
      image: null,
      url: null,
      asset: null,
      direction: null,
      by: null,
    },
    {
      id: 'n-2',
      kind: 'image',
      x: 300,
      y: 20,
      width: 320,
      title: 'Veil gate',
      text: '',
      image: 'concepts/gate.webp',
      url: null,
      asset: null,
      direction: null,
      by: null,
    },
  ],
  edges: [
    {
      id: 'e-1',
      from: 'n-1',
      to: 'n-2',
      fromSide: 'right',
      toSide: 'left',
      kind: 'source',
      label: null,
      by: null,
    },
  ],
});

const summary = (id: string, name: string, edited: string): PlanSummary => ({
  id,
  file: `plans/${id}.plan.json`,
  name,
  preset: 'brainstorm',
  nodes: 3,
  edges: 0,
  images: 1,
  links: 0,
  assets: 0,
  edited,
  problem: null,
});

describe('canvas files and React Flow', () => {
  it('turns a canvas into nodes and edges and back without a change', () => {
    const file = plan();
    const nodes = flowNodesOf(file);
    const edges = flowEdgesOf(file);
    expect(nodes[0]).toMatchObject({
      id: 'n-1',
      type: 'note',
      position: { x: 10, y: 20 },
      width: 220,
    });
    expect(nodes[1]?.width).toBe(320);
    expect(nodes[0]?.ariaLabel).toBe('Note: Frost veil slows time');
    expect(edges[0]).toMatchObject({ source: 'n-1', target: 'n-2', sourceHandle: 'right' });
    expect(samePlan(planOf(file, nodes, edges), file)).toBe(true);
  });

  it('keeps whole pixels and keeps an edge kind the person did not draw', () => {
    const file = plan();
    const nodes = flowNodesOf(file).map(n =>
      n.id === 'n-1' ? { ...n, position: { x: 40.6, y: -3.2 } } : n
    );
    const next = planOf(file, nodes, flowEdgesOf(file));
    expect(next.nodes[0]).toMatchObject({ x: 41, y: -3 });
    expect(next.edges[0]?.kind).toBe('source');
    expect(samePlan(next, file)).toBe(false);
    expect(samePlan({ ...file, nodes: [...file.nodes].reverse() }, file)).toBe(true);
  });

  it('makes nodes and links with ids that sort by time', () => {
    expect(newId('n', 1000, () => 0)).toBe('n-rs000');
    const node = newNode('link', { x: 1.4, y: 2.6 }, { url: 'https://a.example' }, 5);
    expect(node).toMatchObject({ kind: 'link', x: 1, y: 3, url: 'https://a.example', by: null });
    const edge = newEdge(
      { source: 'a', target: 'b', sourceHandle: 'bottom', targetHandle: 'x' },
      5
    );
    expect(edge?.data?.edge).toMatchObject({ kind: 'relates', fromSide: 'bottom', toSide: null });
    expect(
      newEdge({ source: 'a', target: 'a', sourceHandle: null, targetHandle: null }, 5)
    ).toBeNull();
  });

  it('keeps positions and text inside what the file allows', () => {
    const far = newNode('note', { x: 5e6, y: -5e6 }, { text: 'x'.repeat(LIMITS.text + 5) }, 1);
    expect(far).toMatchObject({ x: LIMITS.coord, y: -LIMITS.coord });
    expect(far.text).toHaveLength(LIMITS.text);
    const file = plan();
    const moved = flowNodesOf(file).map(n => ({ ...n, position: { x: -9e9, y: 2.4 } }));
    expect(planOf(file, moved, []).nodes[0]).toMatchObject({ x: -LIMITS.coord, y: 2 });
    expect(coord(1_000_000.4)).toBe(1_000_000);
  });

  it('puts a new node where it covers no other', () => {
    const nodes = flowNodesOf(plan());
    expect(freeSpot([], { x: 5, y: 5 }, 'note')).toEqual({ x: 5, y: 5 });
    const spot = freeSpot(nodes, { x: 10, y: 20 }, 'note');
    expect(spot).not.toEqual({ x: 10, y: 20 });
    const clear = nodes.every(
      n =>
        spot.x + 220 <= n.position.x ||
        n.position.x + (n.width ?? 0) <= spot.x ||
        spot.y + 110 <= n.position.y ||
        n.position.y + 250 <= spot.y
    );
    expect(clear).toBe(true);
  });

  it('reads typed addresses as the file keeps them', () => {
    expect(webAddress('example.com/a')).toBe('https://example.com/a');
    expect(webAddress(' http://localhost:4321 ')).toBe('http://localhost:4321');
    expect(webAddress('javascript:alert(1)')).toBeNull();
    expect(webAddress('two words')).toBeNull();
    expect(webAddress('')).toBeNull();
    expect(hostOf('https://www.example.com/x')).toBe('example.com');
  });

  it('names nodes by title, then their first line', () => {
    const [note, picture] = plan().nodes;
    expect(nodeTitle(note!)).toBe('Frost veil slows time');
    expect(nodeLabel(picture!)).toBe('Picture: Veil gate');
    expect(nodeTitle({ ...note!, text: '' })).toBe('Empty note');
  });
});

describe('the canvas list', () => {
  const list = [
    summary('a', 'Opening story', '2026-10-26T18:00:00.000Z'),
    summary('b', 'Combat ideas', '2026-11-02T08:40:00.000Z'),
  ];

  it('sorts by the last edit and finds by name or file', () => {
    expect(byEdited(list).map(p => p.id)).toEqual(['b', 'a']);
    expect(findPlans(list, 'STORY').map(p => p.id)).toEqual(['a']);
    expect(findPlans(list, 'b.plan').map(p => p.id)).toEqual(['b']);
    expect(findPlans(list, ' ')).toHaveLength(2);
  });

  it('says what a canvas holds', () => {
    expect(contentLine(list[0]!)).toBe('3 nodes · 1 picture');
  });
});
