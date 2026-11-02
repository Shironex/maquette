/**
 * The Plan view in mock mode: each project's canvases kept in memory, with
 * the core's checks that matter to the view (a name, a free id, a canvas
 * that still exists, pictures inside the project, http and https links).
 *
 * `?plans=` in the URL:
 * - `demo`: Frostveil has three canvases; Combat ideas holds notes, two Lab
 *   pictures, a link and the edges between them.
 * - anything else: no canvases yet.
 */
import type { PlanDoc, PlanFile, PlanImage, PlanNode, PlanSummary } from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { GENERATED_POOL } from './lab-fixtures';

type Canvases = Map<string, PlanFile>;

const at = (iso: string) => iso;

const note = (id: string, x: number, y: number, text: string, title = ''): PlanNode => ({
  id,
  kind: 'note',
  x,
  y,
  width: null,
  title,
  text,
  image: null,
  url: null,
  by: null,
});

/** Combat ideas: the canvas the demo opens on. */
export function demoCombat(): PlanFile {
  return {
    version: 1,
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

function demoCanvases(): Canvases {
  const blank = (name: string, edited: string): PlanFile => ({
    version: 1,
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
    ['opening-story', story],
    ['boss-arena', blank('Boss arena', '2026-10-31T12:00:00.000Z')],
  ]);
}

function slugOf(name: string): string {
  const slug = name
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60)
    .replace(/-+$/, '');
  return slug || 'canvas';
}

function checkName(name: string) {
  const trimmed = name.trim();
  if (!trimmed) fail('invalid', 'Give the canvas a name.');
  if (trimmed.length > 120) fail('invalid', 'Keep the canvas name under 120 characters.');
  return trimmed;
}

function checkPlan(plan: PlanFile) {
  checkName(plan.name);
  const ids = new Set(plan.nodes.map(n => n.id));
  for (const node of plan.nodes) {
    if (
      node.kind === 'image' &&
      (!node.image || node.image.startsWith('/') || node.image.includes('..'))
    )
      fail('invalid', `The picture node ${node.id} needs a picture in the project.`);
    if (node.text.length > 20_000 || Math.abs(node.x) > 1_000_000 || Math.abs(node.y) > 1_000_000)
      fail('invalid', `The node ${node.id} is over the canvas limits.`);
    if (node.kind === 'link' && !/^https?:\/\/\S+$/.test(node.url ?? ''))
      fail('invalid', `The link node ${node.id} needs an http or https address.`);
  }
  for (const edge of plan.edges) {
    if (!ids.has(edge.from) || !ids.has(edge.to))
      fail('invalid', `The edge ${edge.id} joins a node the canvas does not have.`);
  }
}

const sortById = <T extends { id: string }>(items: T[]) =>
  [...items].sort((a, b) => a.id.localeCompare(b.id));

export function createPlanHandlers(deps: { root: () => string | null; plans: string | null }) {
  const byProject = new Map<string, Canvases>();
  if (deps.plans === 'demo') byProject.set('frostveil', demoCanvases());
  let picks = 0;

  const canvases = (project: unknown) => {
    const id = String(project);
    let found = byProject.get(id);
    if (!found) {
      found = new Map();
      byProject.set(id, found);
    }
    return found;
  };
  const projectDir = (project: unknown) => `${deps.root() ?? ''}/projects/${String(project)}`;
  const doc = (project: unknown, id: string, plan: PlanFile): PlanDoc => ({
    id,
    file: `plans/${id}.plan.json`,
    plan,
    images: Object.fromEntries(
      plan.nodes
        .filter(n => n.image)
        .map(n => [n.image as string, `${projectDir(project)}/${n.image as string}`])
    ),
  });
  const summary = (id: string, plan: PlanFile): PlanSummary => ({
    id,
    file: `plans/${id}.plan.json`,
    name: plan.name,
    preset: plan.preset,
    nodes: plan.nodes.length,
    edges: plan.edges.length,
    images: plan.nodes.filter(n => n.kind === 'image').length,
    links: plan.nodes.filter(n => n.kind === 'link').length,
    edited: plan.edited,
    problem: null,
  });
  const existing = (project: unknown, id: unknown) => {
    const plan = canvases(project).get(String(id));
    if (!plan) fail('not_found', `Not found: the canvas ${String(id)}`);
    return plan;
  };

  return {
    plan_list: ({ project }: Args) =>
      sortById([...canvases(project)].map(([id, plan]) => summary(id, plan))),
    plan_get: ({ project, id }: Args) => doc(project, String(id), existing(project, id)),
    plan_create: ({ project, name }: Args) => {
      const trimmed = checkName(String(name));
      const all = canvases(project);
      const base = slugOf(trimmed);
      let id = base;
      for (let n = 2; all.has(id); n += 1) id = `${base}-${n}`;
      const now = clock.now().toISOString();
      const plan: PlanFile = {
        version: 1,
        name: trimmed,
        preset: 'brainstorm',
        created: now,
        edited: now,
        nodes: [],
        edges: [],
      };
      all.set(id, plan);
      return doc(project, id, plan);
    },
    plan_save: ({ project, id, plan }: Args) => {
      const current = existing(project, id);
      const next = plan as PlanFile;
      checkPlan(next);
      const saved: PlanFile = {
        ...next,
        // Only Rename changes the name, as in the core.
        name: current.name,
        created: current.created,
        preset: current.preset,
        edited: clock.now().toISOString(),
        nodes: sortById(next.nodes),
        edges: sortById(next.edges),
      };
      canvases(project).set(String(id), saved);
      return doc(project, String(id), saved);
    },
    plan_rename: ({ project, id, name }: Args) => {
      const current = existing(project, id);
      const renamed = {
        ...current,
        name: checkName(String(name)),
        edited: clock.now().toISOString(),
      };
      canvases(project).set(String(id), renamed);
      return summary(String(id), renamed);
    },
    plan_remove: ({ project, id }: Args) => {
      existing(project, id);
      canvases(project).delete(String(id));
      return `${deps.root() ?? ''}/trash/${String(project)}/plans/${String(id)}.plan.json`;
    },
    plan_pick_image: ({ project }: Args): PlanImage => {
      const name = GENERATED_POOL[picks % GENERATED_POOL.length] ?? 'frozen-lake-b.webp';
      picks += 1;
      const path = `concepts/__pool/${name}`;
      return {
        path,
        file: `${projectDir(project)}/${path}`,
        name: name.replace(/\.webp$/, ''),
        copied: false,
      };
    },
    plan_open_link: ({ project, id, node }: Args) => {
      const found = existing(project, id).nodes.find(n => n.id === node);
      if (!found?.url?.startsWith('https://'))
        fail('invalid', 'Only https pages open from a canvas. Copy the address to open it.');
      return null;
    },
  };
}
