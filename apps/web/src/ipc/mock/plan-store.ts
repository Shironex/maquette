/**
 * How the mock stores a canvas, as the core does: a name checked, the
 * fields trimmed, ids sorted and links joining nodes the canvas has.
 */
import type { PlanFile } from '../client';
import { clock } from '@/lib/clock';
import { trimText } from '@/state/plan-text';
import { fail } from './backend-options';

/** `plan` as the core writes it: name, created and preset kept, fields trimmed, ids sorted. */
export function stored(current: PlanFile, next: PlanFile): PlanFile {
  return {
    ...next,
    // Only Rename changes the name, as in the core.
    name: current.name,
    created: current.created,
    preset: current.preset,
    edited: clock.now().toISOString(),
    // Settings in use need version 4, as the core writes them.
    version: next.autoLayout === false || next.fine?.length ? 4 : 2,
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

export function slugOf(name: string): string {
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

export function checkName(name: string) {
  const trimmed = trimText(name);
  if (!trimmed) fail('invalid', 'Give the canvas a name.');
  if (trimmed.length > 120) fail('invalid', 'Keep the canvas name under 120 characters.');
  return trimmed;
}

export function checkPlan(plan: PlanFile) {
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

export function sortById<T extends { id: string }>(items: T[]) {
  return [...items].sort((a, b) => a.id.localeCompare(b.id));
}
