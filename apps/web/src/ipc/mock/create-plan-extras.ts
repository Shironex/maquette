/**
 * A canvas's extras in mock mode, as the core does them
 * (crates/maquette-model/src/plans/removal.rs, start.rs, settings.rs):
 * an agent's removal proposals, which only the person answers; starting a
 * canvas from a node and the nodes linked to it; the "Auto layout after
 * agents" switch; and marking a mechanics overlap as fine. The mock only
 * command `mock_plan_propose` makes an agent's proposal for the visual
 * check, and tells the view as the folder watcher does.
 */
import type {
  ChangeMark,
  FineMark,
  PlanDoc,
  PlanEdge,
  PlanFile,
  PlanNode,
  PlanOp,
  RemovalProposal,
} from '../client';
import { clock } from '@/lib/clock';
import { applyOps } from '@/state/plan-ops';
import { fail, type Args, type BackendOptions } from './backend-options';
import { pairKey, overlapsOf } from './plan-overlap';
import type { MockLogEntry, createMockLogs } from './plan-log';
import { checkName } from './plan-store';

const AGENT = 'Research agent';
const MAX_PER_CALL = 100;

interface ExtrasDeps {
  canvases: (project: unknown) => Map<string, PlanFile>;
  existing: (project: unknown, id: unknown) => PlanFile;
  write: (
    project: unknown,
    id: string,
    next: PlanFile,
    by: string | null,
    as?: { summary: string; mark: ChangeMark; undoes?: string[]; resolves?: string[] }
  ) => PlanDoc;
  doc: (project: unknown, id: string, plan: PlanFile) => PlanDoc;
  logs: ReturnType<typeof createMockLogs>;
  freeId: (project: unknown, name: string) => string;
  emit?: BackendOptions['emit'];
}

/** A node's content without its place, so moving it never makes a proposal stale. */
const contentKey = (node: PlanNode) => JSON.stringify({ ...node, x: 0, y: 0, width: null });

/** The node's name as the core's summaries give it. */
const nameOf = (node: PlanNode) =>
  node.title.trim() ||
  node.text.trim().split('\n')[0]?.trim() ||
  node.asset ||
  node.direction ||
  node.id;

let ids = 0;
const freshId = (prefix: 'n' | 'e') => {
  ids += 1;
  return `${prefix}-s${clock.now().getTime().toString(36)}${ids.toString(36).padStart(3, '0')}`;
};

/** Refuses a person's change to another canvas than the one they opened, as `plan_apply` does. */
function sameCanvas(plan: PlanFile, created: unknown) {
  if (created !== plan.created)
    fail(
      'invalid',
      'This canvas was replaced on disk since it was opened, so your edits were not saved. Open it again from the Plan list; copy anything you need first.'
    );
}

export function createPlanExtras(deps: ExtrasDeps) {
  const { canvases, existing, write, doc, logs } = deps;
  const tell = (project: unknown, id: string) =>
    deps.emit?.({ name: 'plan', project: String(project), id });

  /** The proposals not answered yet, oldest first. */
  const openOnes = (project: unknown, id: unknown) => {
    const all = logs.entries(project, id);
    const answered = new Set(all.flatMap(e => e.resolves ?? []));
    return all.filter(e => e.proposes && !answered.has(e.id));
  };
  const view = (entry: MockLogEntry, plan: PlanFile): RemovalProposal => {
    const asked = new Set((entry.proposes?.nodes ?? []).map(n => n.id));
    const nodes = (entry.proposes?.nodes ?? []).map(asked => {
      const now = plan.nodes.find(n => n.id === asked.id);
      const state = !now ? 'gone' : contentKey(now) === asked.key ? 'here' : 'changed';
      return { id: asked.id, name: asked.name, state } as const;
    });
    return {
      id: entry.id,
      at: entry.at,
      by: entry.by ?? '',
      run: entry.run,
      reason: entry.proposes?.reason ?? '',
      nodes,
      links: plan.edges.filter(e => asked.has(e.from) || asked.has(e.to)).length,
      stale: nodes.some(n => n.state !== 'here'),
    };
  };
  const findOpen = (project: unknown, id: unknown, proposal: unknown) => {
    const found = openOnes(project, id).find(e => e.id === proposal);
    if (!found) fail('not_found', `Not found: the removal proposal ${String(proposal)}`);
    return found;
  };
  const named = (shown: RemovalProposal, one: string, many: string) =>
    shown.nodes.length === 1 && shown.nodes[0] ? one.replace('$', shown.nodes[0].name) : many;

  return {
    plan_proposals: ({ project, id }: Args) => {
      const plan = existing(project, id);
      return openOnes(project, id).map(e => view(e, plan));
    },
    plan_answer_removal: ({ project, id, created, proposal, remove }: Args) => {
      const current = existing(project, id);
      sameCanvas(current, created);
      const shown = view(findOpen(project, id, proposal), current);
      const n = shown.nodes.length;
      if (!remove) {
        const summary = named(
          shown,
          `Kept $, which ${shown.by} asked to remove`,
          `Kept ${n} nodes ${shown.by} asked to remove`
        );
        logs.append(project, id, {
          by: null,
          run: null,
          ops: [],
          summary,
          mark: 'keep',
          resolves: [shown.id],
        });
        return doc(project, String(id), current);
      }
      if (shown.stale)
        fail(
          'invalid',
          'A node of this proposal changed or went since the agent asked, so nothing was removed. Keep it, or ask the agent to propose again.'
        );
      const gone = new Set(shown.nodes.map(x => x.id));
      const ops: PlanOp[] = [
        ...current.edges
          .filter(e => gone.has(e.from) || gone.has(e.to))
          .map(edge => ({ op: 'remove-edge' as const, edge })),
        ...current.nodes
          .filter(x => gone.has(x.id))
          .map(node => ({ op: 'remove-node' as const, node })),
      ];
      const summary = named(
        shown,
        `Removed $, as ${shown.by} asked`,
        `Removed ${n} nodes, as ${shown.by} asked`
      );
      const next = applyOps(current, ops, 'mine').plan;
      return write(project, String(id), next, null, {
        summary,
        mark: 'remove',
        resolves: [shown.id],
      });
    },
    plan_start_from: ({ project, source, node, name }: Args) => {
      const from = existing(project, source);
      const seed = from.nodes.find(x => x.id === node);
      if (!seed) fail('not_found', `Not found: the node ${String(node)} on ${String(source)}`);
      if (seed.kind === 'stop')
        fail(
          'invalid',
          'A camera stop can not start a canvas: stops live only in the camera plan. Start from a note, picture, link, asset or direction instead.'
        );
      const kept = new Set([seed.id]);
      for (const e of from.edges) {
        const other = e.from === seed.id ? e.to : e.to === seed.id ? e.from : null;
        const next = from.nodes.find(x => x.id === other);
        if (next && next.kind !== 'stop' && kept.size < MAX_PER_CALL) kept.add(next.id);
      }
      const fresh = new Map([...kept].map(k => [k, freshId('n')]));
      const nodes = from.nodes.flatMap(x => {
        const nid = fresh.get(x.id);
        return nid ? [{ ...x, id: nid, x: x.x - seed.x, y: x.y - seed.y }] : [];
      });
      const edges = from.edges.flatMap((e): PlanEdge[] => {
        const [a, b] = [fresh.get(e.from), fresh.get(e.to)];
        return a && b ? [{ ...e, id: freshId('e'), from: a, to: b }] : [];
      });
      const title = checkName(String(name ?? '').trim() || nameOf(seed));
      const now = clock.now().toISOString();
      const preset = from.camera ? 'brainstorm' : from.preset;
      const plan: PlanFile = {
        version: 2,
        name: title,
        preset,
        created: now,
        edited: now,
        nodes,
        edges,
      };
      const id = deps.freeId(project, title);
      canvases(project).set(id, plan);
      logs.append(project, id, {
        by: null,
        run: null,
        mark: 'create',
        summary: `Started from ${nameOf(seed)} on ${from.name}`,
        ops: [
          ...nodes.map(n => ({ op: 'add-node' as const, node: n })),
          ...edges.map(edge => ({ op: 'add-edge' as const, edge })),
        ],
      });
      return doc(project, id, plan);
    },
    plan_set_auto_layout: ({ project, id, created, on }: Args) => {
      const current = existing(project, id);
      sameCanvas(current, created);
      if ((current.autoLayout ?? true) === Boolean(on)) return doc(project, String(id), current);
      const next: PlanFile = { ...current, autoLayout: Boolean(on) };
      const summary = `Turned auto layout after agents ${on ? 'on' : 'off'}`;
      return write(project, String(id), next, null, { summary, mark: 'edit' });
    },
    plan_mark_fine: ({ project, id, created, a, b }: Args) => {
      const current = existing(project, id);
      sameCanvas(current, created);
      const [x, y] = [String(a), String(b)].sort();
      const flagged = overlapsOf(current).find(o => o.a === x && o.b === y);
      if (!flagged)
        fail('invalid', 'Those two cards do not overlap now, so there is nothing to mark.');
      if (flagged.fine) fail('invalid', 'That overlap is marked as fine already.');
      const [na, nb] = [x, y].map(k => current.nodes.find(n => n.id === k)!);
      const mark: FineMark = { a: x!, b: y!, key: pairKey(na!, nb!) };
      const fine = [...(current.fine ?? []).filter(m => m.a !== x || m.b !== y), mark];
      const summary = `Marked as fine: ${nameOf(na!)} and ${nameOf(nb!)}`;
      return write(project, String(id), { ...current, fine }, null, { summary, mark: 'edit' });
    },
    // Mock only, for the visual check: an agent asks to remove nodes.
    mock_plan_propose: ({ project, id, nodes, reason }: Args) => {
      const plan = existing(project, id);
      const asked = (nodes as string[]).map(k => {
        const found = plan.nodes.find(n => n.id === k);
        if (!found) fail('not_found', `Not found: the node ${k} on this canvas`);
        return { id: k, name: nameOf(found), key: contentKey(found) };
      });
      const summary =
        asked.length === 1 && asked[0]
          ? `Asked to remove: ${asked[0].name}`
          : `Asked to remove ${asked.length} nodes`;
      const entry = logs.append(project, id, {
        by: AGENT,
        run: 'r-mock',
        ops: [],
        summary,
        mark: 'propose',
        proposes: { nodes: asked, reason: String(reason ?? '') },
      });
      tell(project, String(id));
      return entry.id;
    },
  };
}
