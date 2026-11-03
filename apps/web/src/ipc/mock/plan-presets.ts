/**
 * The starter notes a preset puts on a new canvas in mock mode, a copy of
 * the core's `crates/maquette-model/src/plans/presets.rs` (a test checks
 * them card for card against its `presets/seeds.json`). Brainstorm starts blank; Story is worded for the
 * project's target.
 */
import type { PlanEdge, PlanFile, PlanNode, PlanOp, PlanPreset, ProjectTarget } from '../client';
import { PRESET_NAME } from '@/state/plan-presets';

/** The core's seed grid: 220 pixel notes with room between them. */
const COL = 280;
const ROW = 240;

interface Card {
  col: number;
  row: number;
  title: string;
  text: string;
}

type Join = [from: number, to: number, kind: 'leads-to' | 'down' | 'up'];

const card = (col: number, row: number, title: string, text: string): Card => ({
  col,
  row,
  title,
  text,
});

const STORY_JOINS: readonly Join[] = [
  [2, 3, 'leads-to'],
  [3, 4, 'leads-to'],
  [4, 5, 'leads-to'],
  [0, 2, 'down'],
  [1, 3, 'down'],
  [6, 2, 'up'],
  [7, 5, 'up'],
];

const STORY_GAME: readonly Card[] = [
  card(
    0,
    0,
    'Character: Main character',
    'Who the player is and what they want. Link their asset here once it is on the Board.'
  ),
  card(1, 0, 'Character: Rival', 'Who or what stands in the way, and why.'),
  card(0, 1, 'Beat 1: Opening', 'Where it starts and what the player sees first.'),
  card(1, 1, 'Beat 2: Trouble', 'What goes wrong and pulls the player in.'),
  card(
    2,
    1,
    'Beat 3: Turn',
    'The surprise that changes the plan. Draw two links out of a beat to branch.'
  ),
  card(3, 1, 'Beat 4: Ending', 'How it ends and what the player keeps.'),
  card(
    0,
    2,
    'Place: First place',
    'Where the opening happens. Drop a picture or link a Lab direction.'
  ),
  card(3, 2, 'Place: Last place', 'Where it ends.'),
];

const STORY_WEBSITE: readonly Card[] = [
  card(0, 0, 'Character: Main subject', 'What the site is about: a person, a product or a place.'),
  card(1, 0, 'Character: Visitor', 'Who scrolls through it and what they came for.'),
  card(0, 1, 'Beat 1: First view', 'What a visitor sees before they scroll.'),
  card(1, 1, 'Beat 2: Hook', 'The detail that makes them keep scrolling.'),
  card(
    2,
    1,
    'Beat 3: Reveal',
    'The view the scroll builds up to. Draw two links out of a beat to branch.'
  ),
  card(3, 1, 'Beat 4: Close', 'The last view and what you want them to do next.'),
  card(
    0,
    2,
    'Place: Opening scene',
    'Where the first camera stop looks. Drop a picture or link a Lab direction.'
  ),
  card(3, 2, 'Place: Final scene', 'Where the tour ends.'),
];

const MECHANICS: readonly Card[] = [
  card(
    0,
    0,
    'System: Core action',
    'Verb: what the player does most\nLook: how it reads on screen\nInput: the key or button'
  ),
  card(
    1,
    0,
    'System: Movement',
    'Verb: how the player gets around\nLook: how it reads on screen\nInput: the key or button'
  ),
  card(
    2,
    0,
    'System: Defence',
    'Verb: how the player stays alive\nLook: how it reads on screen\nInput: the key or button'
  ),
  card(
    0,
    1,
    'Keep them apart',
    'One verb, one look and one input per system. Two systems that share any of them read as one. Add a card for each new system.'
  ),
];

/** Ids that sort in seed order, as the core's do. */
const idOf = (prefix: 'n' | 'e', nowMs: number, i: number) =>
  `${prefix}-${nowMs.toString(36)}${i.toString(36).padStart(3, '0')}`;

const SIDES = {
  'leads-to': ['right', 'left'],
  down: ['bottom', 'top'],
  up: ['top', 'bottom'],
} as const;

/** The starter nodes and links of `preset` for a project of `target`, made by `by`. */
export function seedOf(
  preset: PlanPreset,
  target: ProjectTarget,
  nowMs: number,
  by: string | null = null
): { nodes: PlanNode[]; edges: PlanEdge[] } {
  const cards =
    preset === 'story'
      ? target === 'game'
        ? STORY_GAME
        : STORY_WEBSITE
      : preset === 'mechanics'
        ? MECHANICS
        : [];
  const joins = preset === 'story' ? STORY_JOINS : [];
  const nodes: PlanNode[] = cards.map((c, i) => ({
    id: idOf('n', nowMs, i),
    kind: 'note',
    x: c.col * COL,
    y: c.row * ROW,
    width: null,
    title: c.title,
    text: c.text,
    image: null,
    url: null,
    asset: null,
    direction: null,
    by,
  }));
  const edges: PlanEdge[] = joins.map(([from, to, kind], i) => {
    const [fromSide, toSide] = SIDES[kind];
    return {
      id: idOf('e', nowMs, i),
      from: nodes[from]?.id ?? '',
      to: nodes[to]?.id ?? '',
      fromSide,
      toSide,
      kind: kind === 'leads-to' ? 'leads-to' : 'relates',
      label: null,
      by,
    };
  });
  return { nodes, edges };
}

const byId = <T extends { id: string }>(items: T[]) =>
  [...items].sort((a, b) => a.id.localeCompare(b.id));

/** A new canvas called `name` (already checked and trimmed) with its preset's starter notes. */
export function seededPlan(
  name: string,
  preset: PlanPreset,
  target: ProjectTarget,
  now: Date
): PlanFile {
  const seed = seedOf(preset, target, now.getTime());
  return {
    version: 2,
    name,
    preset,
    created: now.toISOString(),
    edited: now.toISOString(),
    nodes: byId(seed.nodes),
    edges: byId(seed.edges),
  };
}

/** The first log line of `plan`: an add for each starter, so one undo takes them back. */
export function madeLine(plan: PlanFile): { ops: PlanOp[]; summary: string } {
  return {
    ops: [
      ...plan.nodes.map(node => ({ op: 'add-node' as const, node })),
      ...plan.edges.map(edge => ({ op: 'add-edge' as const, edge })),
    ],
    summary: plan.nodes.length
      ? `Made the canvas ${plan.name} from the ${PRESET_NAME[plan.preset]} preset`
      : `Made the canvas ${plan.name}`,
  };
}
