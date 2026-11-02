/**
 * The Plan view in mock mode: each project's canvases kept in memory, with
 * the core's checks that matter to the view (a name, a free id, a canvas
 * that still exists, pictures inside the project, http and https links).
 *
 * `?plans=` in the URL:
 * - `demo`: Frostveil's canvases (see plan-fixtures.ts), with asset and Lab
 *   direction nodes read live by `plan_links` (create-plan-links.ts).
 * - `locked`: the same canvases, with every save failing.
 * - anything else: no canvases yet.
 *
 * Saves arrive as operations (`plan_apply`) and are laid over the canvas as
 * the core lays them, each write logged for the Changes panel and undo
 * (plan-log.ts). Two mock only commands serve the visual check:
 * `mock_plan_agent` writes an agent's run into a canvas and
 * `mock_plan_outside` changes a node's text as another computer's sync
 * would; both tell the view as the folder watcher does.
 */
import type { ChangeMark, PlanDoc, PlanFile, PlanImage, PlanOp, PlanSummary } from '../client';
import { clock } from '@/lib/clock';
import { applyOps, diffPlans, invertOps } from '@/state/plan-ops';
import { CAMERA_PLAN_ID } from '@/state/plan-camera';
import { trimText } from '@/state/plan-text';
import { fail, type Args, type BackendOptions } from './backend-options';
import { createCameraHandlers } from './create-camera-handlers';
import { createPlanLinks, type PlanLinkDeps } from './create-plan-links';
import { GENERATED_POOL } from './lab-fixtures';
import { demoCanvases, demoLog, note, type Canvases } from './plan-fixtures';
import { createMockLogs, summaryOf } from './plan-log';

/** The agent `mock_plan_agent` writes as. */
const AGENT = 'Research agent';
const AGENT_NOTES = [
  { title: 'Dusk light that reads on phones', text: 'Keep the veil glow above 40% brightness.' },
  { title: 'Stop 1 crops the vending machine', text: 'Frame the gate, not the street.' },
  { title: 'Rune burst on the third hit', text: 'A short freeze frame sells it.' },
];

/** `plan` as the core writes it: name, created and preset kept, fields trimmed, ids sorted. */
function stored(current: PlanFile, next: PlanFile): PlanFile {
  return {
    ...next,
    // Only Rename changes the name, as in the core.
    name: current.name,
    created: current.created,
    preset: current.preset,
    edited: clock.now().toISOString(),
    version: 2,
    nodes: sortById(
      next.nodes.map(n => ({
        ...n,
        title: trimText(n.title),
        url: n.url === null ? null : trimText(n.url),
      }))
    ),
    edges: sortById(
      next.edges.map(e => ({
        ...e,
        label: e.label === null ? null : trimText(e.label) || null,
      }))
    ),
  };
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
  const trimmed = trimText(name);
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
    if (node.kind === 'asset' && !/^[A-Za-z0-9_-]{1,128}$/.test(node.asset ?? ''))
      fail('invalid', `The asset node ${node.id} needs the id of an asset in the project.`);
    if (node.kind === 'direction' && !/^[A-Za-z0-9_-]{1,128}$/.test(node.direction ?? ''))
      fail('invalid', `The Lab direction node ${node.id} needs the id of a direction in the Lab.`);
    if ((node.asset && node.kind !== 'asset') || (node.direction && node.kind !== 'direction'))
      fail('invalid', `Only an asset or Lab direction node links one (${node.id}).`);
  }
  for (const edge of plan.edges) {
    if (!ids.has(edge.from) || !ids.has(edge.to))
      fail('invalid', `The edge ${edge.id} joins a node the canvas does not have.`);
  }
}

function sortById<T extends { id: string }>(items: T[]) {
  return [...items].sort((a, b) => a.id.localeCompare(b.id));
}

export function createPlanHandlers(
  deps: PlanLinkDeps & {
    plans: string | null;
    /** Tells the view a canvas's file or log changed, as the watcher does. */
    emit?: BackendOptions['emit'];
  }
) {
  const byProject = new Map<string, Canvases>();
  const logs = createMockLogs();
  // `locked` serves the demo canvases but every save fails, as a file another program holds would.
  if (deps.plans === 'demo' || deps.plans === 'locked') {
    byProject.set('frostveil', demoCanvases());
    for (const entry of demoLog()) logs.append('frostveil', 'combat-ideas', entry);
  }
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
    assets: plan.nodes.filter(n => n.kind === 'asset').length,
    stops: plan.nodes.filter(n => n.kind === 'stop').length,
    camera: Boolean(plan.camera),
    edited: plan.edited,
    problem: null,
  });
  const existing = (project: unknown, id: unknown) => {
    const plan = canvases(project).get(String(id));
    if (!plan) fail('not_found', `Not found: the canvas ${String(id)}`);
    return plan;
  };
  /** Checks and writes `next` over the canvas, logs what changed and tells the view. */
  const write = (
    project: unknown,
    id: string,
    next: PlanFile,
    by: string | null,
    as?: { summary: string; mark: ChangeMark; undoes: string[] }
  ) => {
    const current = existing(project, id);
    checkPlan(next);
    const saved = stored(current, next);
    const ops = diffPlans(current, saved);
    canvases(project).set(id, saved);
    if (ops.length) {
      const run = by ? 'r-mock' : null;
      logs.append(project, id, { by, run, ops, ...(as ?? summaryOf(ops)) });
      deps.emit?.({ name: 'plan', project: String(project), id });
    }
    return doc(project, id, saved);
  };

  return {
    ...createPlanLinks(deps),
    ...createCameraHandlers({
      get: (project, id) => canvases(project).get(id),
      put: (project, id, plan) => void canvases(project).set(id, plan),
      doc,
      logs,
      layout: deps.live.layout,
      emit: deps.emit,
      projectDir,
    }),
    plan_list: ({ project }: Args) =>
      sortById([...canvases(project)].map(([id, plan]) => summary(id, plan))),
    plan_get: ({ project, id }: Args) => doc(project, String(id), existing(project, id)),
    plan_create: ({ project, name }: Args) => {
      const trimmed = checkName(String(name));
      const all = canvases(project);
      const base = slugOf(trimmed);
      let id = base;
      // The camera plan's id is kept for it, as the core keeps it.
      for (let n = 2; all.has(id) || id === CAMERA_PLAN_ID; n += 1) id = `${base}-${n}`;
      const now = clock.now().toISOString();
      const plan: PlanFile = {
        version: 2,
        name: trimmed,
        preset: 'brainstorm',
        created: now,
        edited: now,
        nodes: [],
        edges: [],
      };
      all.set(id, plan);
      logs.append(project, id, {
        by: null,
        run: null,
        ops: [],
        summary: `Made the canvas ${trimmed}`,
        mark: 'create',
      });
      return doc(project, id, plan);
    },
    plan_apply: ({ project, id, ops }: Args) => {
      const current = existing(project, id);
      if (deps.plans === 'locked')
        fail('io', 'The canvas file is in use by another program, so it could not be written.');
      return write(project, String(id), applyOps(current, ops as PlanOp[], 'mine').plan, null);
    },
    plan_log: ({ project, id, limit }: Args) => {
      existing(project, id);
      return logs.lines(project, id, Number(limit ?? 200));
    },
    plan_undo: ({ project, id, changes }: Args) => {
      const current = existing(project, id);
      const wanted = new Set(changes as string[]);
      const all = logs.entries(project, id);
      const undone = new Set(all.flatMap(e => e.undoes));
      const picked = all.filter(e => wanted.has(e.id));
      if (picked.some(e => e.undoes.length))
        fail('invalid', 'That change is an undo, and an undo can not be undone.');
      if (picked.length !== wanted.size || picked.some(e => undone.has(e.id) || !e.ops.length))
        fail('invalid', 'That change cannot be undone any more.');
      // Newest first, each carefully: a field changed since stays.
      const ops = [...picked].reverse().flatMap(e => invertOps(e.ops));
      const next = applyOps(current, ops, 'careful').plan;
      const only = picked.length === 1 ? picked[0] : undefined;
      const what = only ? only.summary : `${picked.length} changes`;
      const changed = diffPlans(current, stored(current, next)).length > 0;
      // Nothing left to undo still writes its line, so the change counts as undone.
      if (!changed)
        logs.append(project, id, {
          by: null,
          run: null,
          ops: [],
          summary: `Nothing to undo, already changed: ${what}`,
          mark: 'undo',
          undoes: [...wanted],
        });
      const summary = only ? `Undid: ${what}` : `Undid ${what}`;
      const saved = changed
        ? write(project, String(id), next, null, { summary, mark: 'undo', undoes: [...wanted] })
        : doc(project, String(id), current);
      return { doc: saved, changed, kept: 0 };
    },
    plan_keep: ({ project, id, changes, confirmed }: Args) => {
      existing(project, id);
      const clicked = changes as string[];
      const seen = [...((confirmed as string[] | undefined) ?? [])].sort();
      const listed = logs
        .needs(project, id, clicked)
        .map(n => n.id)
        .sort();
      if (seen.join() !== listed.join())
        fail('invalid', 'The list changed since you saw it, so nothing was kept. Review again.');
      const ids = [...clicked, ...seen];
      const keepable = new Set(
        logs
          .lines(project, id, 1000)
          .filter(l => l.keepable)
          .map(l => l.id)
      );
      if (!ids.length || ids.some(c => !keepable.has(c)))
        fail('invalid', 'That change has nothing to keep.');
      if (logs.needs(project, id, ids).length)
        fail('invalid', 'Keeping these also needs the earlier changes under them kept.');
      const only =
        ids.length === 1 ? logs.entries(project, id).find(e => e.id === ids[0]) : undefined;
      const summary = only ? `Kept: ${only.summary}` : `Kept ${ids.length} agent changes`;
      logs.append(project, id, { by: null, run: null, ops: [], summary, mark: 'keep', keeps: ids });
      return null;
    },
    plan_keep_needs: ({ project, id, changes }: Args) => {
      existing(project, id);
      return logs.needs(project, id, changes as string[]);
    },
    plan_watch: () => true,
    // Mock only, for the visual check: an agent's run and a change from another computer.
    mock_plan_agent: ({ project, id, edit }: Args) => {
      const current = existing(project, id);
      // With `edit`, only that node's text, as a second call of the same run.
      if (edit) {
        const nodes = current.nodes.map(n =>
          n.id === edit ? { ...n, text: `${n.text} Check it on a phone first.` } : n
        );
        return write(project, String(id), { ...current, nodes }, AGENT);
      }
      const right = Math.max(...current.nodes.map(n => n.x + (n.width ?? 260)), 0) + 64;
      const add = AGENT_NOTES.map((n, i) => ({
        ...note(`n-a${i + 1}`, right, 60 + i * 170, n.text, n.title),
        by: AGENT,
      }));
      const edited = current.nodes.map(n =>
        n.id === 'n-06' ? { ...n, text: `${n.text} Keep it under the lantern line.` } : n
      );
      const next = { ...current, nodes: [...edited, ...add] };
      return write(project, String(id), next, AGENT);
    },
    mock_plan_outside: ({ project, id, node, text }: Args) => {
      const current = existing(project, id);
      const next = {
        ...current,
        nodes: current.nodes.map(n => (n.id === node ? { ...n, text: String(text) } : n)),
      };
      canvases(project).set(String(id), stored(current, next));
      deps.emit?.({ name: 'plan', project: String(project), id: String(id) });
      return null;
    },
    plan_rename: ({ project, id, name }: Args) => {
      const current = existing(project, id);
      const renamed = {
        ...current,
        name: checkName(String(name)),
        edited: clock.now().toISOString(),
      };
      canvases(project).set(String(id), renamed);
      logs.append(project, id, {
        by: null,
        run: null,
        ops: [],
        summary: `Renamed to ${renamed.name}`,
        mark: 'rename',
      });
      return summary(String(id), renamed);
    },
    plan_remove: ({ project, id }: Args) => {
      existing(project, id);
      canvases(project).delete(String(id));
      logs.drop(project, id);
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
