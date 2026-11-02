/**
 * The planning canvases mock mode's `?plans=demo` serves for Frostveil:
 * Combat ideas (notes, two Lab pictures, a link and the edges between
 * them), Boss: Frost Warden (assets of the Board and a Lab direction,
 * linked live, with one asset that is no longer on the Board), Opening
 * story and an empty Boss arena.
 */
import type { InboxItem, PlanFile, PlanNode } from '../client';
import type { MockLogEntry } from './plan-log';

export type Canvases = Map<string, PlanFile>;

const at = (iso: string) => iso;

export const note = (id: string, x: number, y: number, text: string, title = ''): PlanNode => ({
  id,
  kind: 'note',
  x,
  y,
  width: null,
  title,
  text,
  image: null,
  url: null,
  asset: null,
  direction: null,
  by: null,
});

/** A node linking the asset or Lab direction `id`: only the id is kept. */
const linked = (
  kind: 'asset' | 'direction',
  nodeId: string,
  x: number,
  y: number,
  id: string
): PlanNode => ({ ...note(nodeId, x, y, ''), kind, [kind]: id });

const edge = (
  id: string,
  from: string,
  to: string,
  kind: 'relates' | 'source' = 'relates',
  label: string | null = null
) => ({
  id,
  from,
  to,
  fromSide: 'right' as const,
  toSide: 'left' as const,
  kind,
  label,
  by: null,
});

/** Combat ideas: the canvas the demo opens on. */
export function demoCombat(): PlanFile {
  return {
    version: 2,
    name: 'Combat ideas',
    preset: 'brainstorm',
    created: at('2026-10-30T09:00:00.000Z'),
    edited: at('2026-11-02T08:40:00.000Z'),
    nodes: [
      note(
        'n-01',
        60,
        80,
        'Slow time around you for a moment. Hold Shift, the same key as Dash.',
        'Frost veil'
      ),
      note(
        'n-02',
        60,
        300,
        'Every third hit leaves a rune. Three runes in a row burst.',
        'Rune combos'
      ),
      {
        ...note('n-03', 380, 40, '', 'Veil gate at night'),
        kind: 'image',
        width: 260,
        image: 'concepts/__pool/veil-gate-kit-b.webp',
      },
      {
        ...note('n-04', 380, 330, '', 'Frozen lake arena'),
        kind: 'image',
        image: 'concepts/__pool/frozen-lake-d.webp',
      },
      {
        ...note(
          'n-05',
          720,
          120,
          'Warm key, cool fill, glow under a third of the frame.',
          'Lighting small scenes at night'
        ),
        kind: 'link',
        url: 'https://example.com/lighting-small-scenes',
      },
      note('n-06', 720, 330, 'Does the veil read on a phone screen?'),
    ],
    edges: [
      {
        id: 'e-01',
        from: 'n-01',
        to: 'n-03',
        fromSide: 'right',
        toSide: 'left',
        kind: 'relates',
        label: null,
        by: null,
      },
      {
        id: 'e-02',
        from: 'n-02',
        to: 'n-04',
        fromSide: 'right',
        toSide: 'left',
        kind: 'relates',
        label: null,
        by: null,
      },
      {
        id: 'e-03',
        from: 'n-03',
        to: 'n-05',
        fromSide: 'right',
        toSide: 'left',
        kind: 'source',
        label: null,
        by: null,
      },
    ],
  };
}

/** Boss: Frost Warden, the asset links: live, no model, to check, a file gone and a removed asset. */
export function demoBoss(): PlanFile {
  return {
    version: 2,
    name: 'Boss: Frost Warden',
    preset: 'brainstorm',
    created: at('2026-10-31T09:00:00.000Z'),
    edited: at('2026-11-01T17:00:00.000Z'),
    nodes: [
      linked('direction', 'n-01', 40, 60, 'frozen-lake-arena'),
      note('n-02', 40, 330, 'Slow and heavy. Its stance breaks only while the frost veil is up.'),
      linked('asset', 'n-03', 340, 60, 'elsheria'),
      linked('asset', 'n-04', 340, 230, 'veil-gate'),
      linked('asset', 'n-05', 340, 400, 'frost-warden'),
      linked('asset', 'n-06', 660, 60, 'rune-obelisk'),
      linked('asset', 'n-07', 660, 230, 'observatory-arch'),
      note('n-08', 660, 400, 'Big shape that reads from the far camera.'),
    ],
    edges: [
      edge('e-01', 'n-01', 'n-03', 'relates', 'arena'),
      edge('e-02', 'n-02', 'n-05', 'relates', 'brief'),
      edge('e-03', 'n-03', 'n-06'),
    ],
  };
}

export function demoCanvases(): Canvases {
  const blank = (name: string, edited: string): PlanFile => ({
    version: 2,
    name,
    preset: 'brainstorm',
    created: edited,
    edited,
    nodes: [],
    edges: [],
  });
  const story = blank('Opening story', '2026-10-26T18:00:00.000Z');
  story.nodes = [note('n-01', 0, 0, 'The Warden wakes and the gate shuts.', 'Beat 1')];
  return new Map([
    ['combat-ideas', demoCombat()],
    ['boss-frost-warden', demoBoss()],
    ['opening-story', story],
    ['boss-arena', blank('Boss arena', '2026-10-31T12:00:00.000Z')],
  ]);
}

/** Combat ideas' change log: made, an agent's link a day ago, then the person's move. */
export function demoLog(): Omit<MockLogEntry, 'id'>[] {
  const combat = demoCombat();
  const link = combat.nodes.find(n => n.id === 'n-05') as PlanNode;
  const edgeTo = combat.edges.find(e => e.id === 'e-03');
  const frost = combat.nodes.find(n => n.id === 'n-01') as PlanNode;
  const base = { undoes: [] };
  return [
    {
      ...base,
      at: '2026-10-30T09:00:00.000Z',
      by: null,
      run: null,
      summary: 'Made the canvas Combat ideas',
      mark: 'create',
      ops: [],
    },
    {
      ...base,
      at: '2026-11-01T16:20:00.000Z',
      by: 'Research agent',
      run: 'r-old',
      summary: 'Added link: Lighting small scenes at night',
      mark: 'add',
      ops: [
        { op: 'add-node', node: link },
        ...(edgeTo ? [{ op: 'add-edge' as const, edge: edgeTo }] : []),
      ],
    },
    {
      ...base,
      at: '2026-11-02T08:40:00.000Z',
      by: null,
      run: null,
      summary: 'Moved 1 note',
      mark: 'move',
      ops: [{ op: 'set-node', before: { ...frost, y: 40 }, after: frost }],
    },
  ];
}

/** The Inbox's rune obelisk, attached to the Board's Rune obelisk, so its node has a model to check. */
export function aimDemoInbox(items: InboxItem[]): InboxItem[] {
  return items.map(i =>
    i.id === 'rune_obelisk.glb'
      ? { ...i, aim: { project: 'frostveil', asset: 'rune-obelisk', assetName: 'Rune obelisk' } }
      : i
  );
}
