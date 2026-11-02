/**
 * The Plan view in mock mode: each project's canvases kept in memory, with
 * the core's checks that matter to the view (a name, a free id, a canvas
 * that still exists, pictures inside the project, http and https links).
 *
 * `?plans=` in the URL:
 * - `demo`: Frostveil's canvases (see plan-fixtures.ts), with asset and Lab
 *   direction nodes read live by `plan_links` (create-plan-links.ts).
 * - anything else: no canvases yet.
 */
import type { PlanDoc, PlanFile, PlanImage, PlanSummary } from '../client';
import { clock } from '@/lib/clock';
import { trimText } from '@/state/plan-text';
import { fail, type Args } from './backend-options';
import { createPlanLinks, type PlanLinkDeps } from './create-plan-links';
import { GENERATED_POOL } from './lab-fixtures';
import { demoCanvases, type Canvases } from './plan-fixtures';

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

const sortById = <T extends { id: string }>(items: T[]) =>
  [...items].sort((a, b) => a.id.localeCompare(b.id));

export function createPlanHandlers(deps: PlanLinkDeps & { plans: string | null }) {
  const byProject = new Map<string, Canvases>();
  // `locked` serves the demo canvases but every save fails, as a file another program holds would.
  if (deps.plans === 'demo' || deps.plans === 'locked') byProject.set('frostveil', demoCanvases());
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
    edited: plan.edited,
    problem: null,
  });
  const existing = (project: unknown, id: unknown) => {
    const plan = canvases(project).get(String(id));
    if (!plan) fail('not_found', `Not found: the canvas ${String(id)}`);
    return plan;
  };

  return {
    ...createPlanLinks(deps),
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
        version: 2,
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
      if (deps.plans === 'locked')
        fail('io', 'The canvas file is in use by another program, so it could not be written.');
      const saved: PlanFile = {
        ...next,
        // Only Rename changes the name, as in the core.
        name: current.name,
        created: current.created,
        preset: current.preset,
        edited: clock.now().toISOString(),
        // Trimmed as the core trims a save.
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
