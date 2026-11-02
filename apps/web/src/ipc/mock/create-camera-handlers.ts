/**
 * The camera plan in mock mode, as the core keeps it: made from the mock
 * `layout.json` the first time, its main line reviewed against the layout
 * with the exact diff, applied only with the keys of the review shown (a
 * stale one is refused), an Apply undone only while the layout is as it
 * left it, and a rebase or start over after the layout changed. The mock
 * only command `mock_layout_outside` changes the layout's stops as the
 * Inspector or an agent's set_layout would, for the stale state.
 */
import type {
  CameraBase,
  CameraReview,
  CameraView,
  LayoutStop,
  PlanDoc,
  PlanFile,
  PlanNode,
  RebaseReport,
  SceneLayout,
} from '../client';
import { clock } from '@/lib/clock';
import { diffPlans } from '@/state/plan-ops';
import { fail, type Args, type BackendOptions } from './backend-options';
import type { createFakeLayout } from './layout';
import { chainsOf, diffStops, keyOf, mainOf, stopsOf } from './plan-camera-diff';
import { posterPath } from './create-page-handlers';
import type { createMockLogs } from './plan-log';

const ID = 'camera-plan';
const STEP = 280;

export interface CameraDeps {
  get: (project: unknown, id: string) => PlanFile | undefined;
  put: (project: unknown, id: string, plan: PlanFile) => void;
  doc: (project: unknown, id: string, plan: PlanFile) => PlanDoc;
  logs: ReturnType<typeof createMockLogs>;
  layout?: ReturnType<typeof createFakeLayout>;
  emit?: BackendOptions['emit'];
  projectDir: (project: unknown) => string;
}

/** Frostveil's stops in the order the mock posters were captured. */
const POSTERED = ['arrival', 'veil-gate', 'crystals', 'keystone', 'pullback'];

const stopNode = (stop: LayoutStop, x: number, y: number, n: number): PlanNode => ({
  id: `n-stop-${n.toString(36)}`,
  kind: 'stop',
  x,
  y,
  width: null,
  title: '',
  text: '',
  image: null,
  url: null,
  asset: null,
  direction: null,
  by: null,
  stop,
});

let made = 0;

/** A line of stop nodes for `stops` from `(x, y)`, joined in order. */
function lineOf(stops: LayoutStop[], x: number, y: number) {
  const nodes = stops.map((s, i) => stopNode(s, x + i * STEP, y, (made += 1)));
  const edges = nodes.slice(1).map((n, i) => ({
    id: `e-lead-${(made += 1).toString(36)}`,
    from: nodes[i]!.id,
    to: n.id,
    fromSide: 'right' as const,
    toSide: 'left' as const,
    kind: 'leads-to' as const,
    label: null,
    by: null,
  }));
  return { nodes, edges };
}

export function createCameraHandlers(deps: CameraDeps) {
  const history = new Map<string, { before: SceneLayout; after: string }>();
  const layoutOf = (project: unknown): SceneLayout => {
    if (!deps.layout) fail('unavailable', 'Mock mode has no layout here.');
    return deps.layout.view(String(project)).layout;
  };
  const tell = (project: unknown) => {
    deps.emit?.({ name: 'plan', project: String(project), id: ID });
    deps.emit?.({ name: 'layout', project: String(project) });
  };
  /** Writes the plan as the person, with its log line carrying the base's change. */
  const logged = (project: unknown, next: PlanFile, summary: string, mark: 'apply' | 'rebase') => {
    const current = ensure(project);
    const saved = { ...next, edited: clock.now().toISOString(), version: 3 };
    deps.put(project, ID, saved);
    const ops = [
      { op: 'set-camera' as const, before: current.camera ?? null, after: saved.camera ?? null },
      ...diffPlans(current, saved),
    ];
    return deps.logs.append(project, ID, { by: null, run: null, ops, summary, mark });
  };
  const ensure = (project: unknown): PlanFile => {
    const found = deps.get(project, ID);
    if (found) return found;
    const stops = layoutOf(project).stops;
    const now = clock.now().toISOString();
    const plan: PlanFile = {
      version: 3,
      name: 'Camera plan',
      preset: 'brainstorm',
      created: now,
      edited: now,
      ...lineOf(stops, 0, 0),
      camera: { stops },
    };
    deps.put(project, ID, plan);
    deps.logs.append(project, ID, {
      by: null,
      run: null,
      ops: [],
      summary: `Made the camera plan from layout.json (${stops.length} stops)`,
      mark: 'create',
    });
    return plan;
  };
  const view = (project: unknown): CameraView => {
    const plan = ensure(project);
    const layout = layoutOf(project);
    const base = plan.camera?.stops ?? [];
    const main = mainOf(plan);
    return {
      doc: deps.doc(project, ID, plan),
      posters: POSTERED.map((stop, i) => ({
        stop,
        poster: posterPath(deps.projectDir(project), i),
      })),
      main: main?.nodes ?? [],
      problem: main?.problem ?? null,
      lines: chainsOf(plan)
        .filter(c => c.nodes.join() !== main?.nodes.join())
        .map(({ nodes, agent, problem }) => ({ nodes, agent, problem })),
      stale: JSON.stringify(base) !== JSON.stringify(layout.stops),
      outside: diffStops(base, layout.stops),
    };
  };
  const review = (project: unknown): { review: CameraReview; stops: LayoutStop[] } => {
    const plan = ensure(project);
    const layout = layoutOf(project);
    const main = mainOf(plan);
    const stops = stopsOf(plan, main);
    const diff = diffStops(layout.stops, stops, main?.nodes ?? []);
    const stale = JSON.stringify(plan.camera?.stops ?? []) !== JSON.stringify(layout.stops);
    const problems: string[] = [];
    if (main?.problem) problems.push(main.problem);
    if (!stops.length) problems.push('The plan has no line of stops to apply. Add a stop first.');
    if (stale)
      problems.push(
        'layout.json changed since this plan was based on it. Compare or rebase first, then review again.'
      );
    if (!problems.length && diff.added + diff.removed + diff.moved + diff.changed === 0)
      problems.push('Nothing to apply: layout.json already has this line.');
    return {
      review: {
        diff,
        chain: main?.nodes ?? [],
        dropped: 0,
        stale,
        problems,
        layoutKey: keyOf(layout),
        planKey: keyOf([main?.nodes ?? [], stops]),
      },
      stops,
    };
  };
  const rebased = (plan: PlanFile, now: LayoutStop[], mode: 'rebase' | 'start-over') => {
    const report: RebaseReport = { taken: [], added: [], removed: [], clashes: [], change: null };
    const base = plan.camera?.stops ?? [];
    const main = mainOf(plan);
    const head = plan.nodes.find(n => n.id === main?.nodes[0]) ?? { x: 0, y: 0 };
    const mine = new Set(
      mode === 'start-over'
        ? plan.nodes.filter(n => n.stop && !n.by).map(n => n.id)
        : (main?.nodes ?? [])
    );
    const kept = stopsOf(plan, main).flatMap(stop => {
      const was = base.find(s => s.id === stop.id);
      const fresh = now.find(s => s.id === stop.id);
      if (!fresh) {
        if (was && JSON.stringify(was) === JSON.stringify(stop)) {
          report.removed.push(stop.name);
          return [];
        }
        return [stop];
      }
      if (!was) return [stop];
      const next = { ...stop };
      for (const key of ['name', 'position', 'target', 'fov', 'models'] as const) {
        const [b, p, l] = [was[key], stop[key], fresh[key]].map(v => JSON.stringify(v));
        if (l === b) continue;
        if (p === b) {
          Object.assign(next, { [key]: fresh[key] });
          report.taken.push(`${stop.name}: ${key}`);
        } else if (p !== l)
          report.clashes.push(`${stop.name}: ${key} changed in both; the plan's value stays`);
      }
      return [next];
    });
    const fresh = now.filter(s => !base.some(b => b.id === s.id));
    report.added = fresh.map(s => s.name);
    const stops = mode === 'start-over' ? now : [...kept, ...fresh];
    const line = lineOf(stops, head.x, head.y);
    const next: PlanFile = {
      ...plan,
      nodes: [...plan.nodes.filter(n => !mine.has(n.id)), ...line.nodes],
      edges: [...plan.edges.filter(e => !mine.has(e.from) && !mine.has(e.to)), ...line.edges],
      camera: { stops: now } satisfies CameraBase,
    };
    return {
      next,
      report:
        mode === 'start-over'
          ? { ...report, taken: [], removed: [], clashes: [], added: [] }
          : report,
    };
  };

  return {
    plan_camera: ({ project }: Args) => view(project),
    plan_camera_review: ({ project }: Args) => review(project).review,
    plan_camera_apply: ({ project, layoutKey, planKey }: Args) => {
      const { review: now, stops } = review(project);
      if (now.layoutKey !== layoutKey)
        fail(
          'invalid',
          'layout.json changed since this review was shown, so nothing was applied. Review again to see the changes as they are now.'
        );
      if (now.planKey !== planKey)
        fail(
          'invalid',
          'The camera plan changed since this review was shown, so nothing was applied. Review again.'
        );
      if (now.problems.length) fail('invalid', now.problems.join(' '));
      const before = layoutOf(project);
      deps.layout?.save(String(project), { ...before, stops });
      const after = layoutOf(project);
      const plan = ensure(project);
      const counts = `${now.diff.added} stop${now.diff.added === 1 ? '' : 's'} added · ${now.diff.moved} moved · ${now.diff.removed} removed · ${now.diff.changed} changed`;
      const entry = logged(
        project,
        { ...plan, camera: { stops } },
        `Applied the camera plan to layout.json: ${counts}`,
        'apply'
      );
      history.set(entry.id, { before, after: keyOf(after) });
      tell(project);
      return view(project);
    },
    plan_camera_undo_apply: ({ project, change }: Args) => {
      const kept = history.get(String(change));
      if (!kept) fail('invalid', `The change ${String(change)} is not an Apply.`);
      if (deps.logs.entries(project, ID).some(e => e.undoes.includes(String(change))))
        fail('invalid', 'That Apply was undone already.');
      if (keyOf(layoutOf(project)) !== kept.after)
        fail(
          'invalid',
          'layout.json changed since this Apply, so it was not put back. Undo the later change first, or edit the stops by hand.'
        );
      deps.layout?.save(String(project), kept.before);
      const plan = ensure(project);
      const saved = {
        ...plan,
        camera: { stops: kept.before.stops },
        edited: clock.now().toISOString(),
      };
      deps.put(project, ID, saved);
      deps.logs.append(project, ID, {
        by: null,
        run: null,
        ops: [{ op: 'set-camera', before: plan.camera ?? null, after: saved.camera }],
        summary: 'Undid: the Apply to layout.json',
        mark: 'undo',
        undoes: [String(change)],
      });
      tell(project);
      return view(project);
    },
    plan_camera_rebase: ({ project, mode, dry }: Args) => {
      const plan = ensure(project);
      const how = mode === 'start-over' ? 'start-over' : 'rebase';
      const { next, report } = rebased(plan, layoutOf(project).stops, how);
      if (!dry) {
        const summary =
          how === 'start-over'
            ? 'Started the camera plan over from layout.json'
            : `Rebased the camera plan on layout.json: ${report.taken.length} taken, ${report.added.length} added, ${report.removed.length} removed, ${report.clashes.length} kept from the plan`;
        report.change = logged(project, next, summary, 'rebase').id;
        deps.emit?.({ name: 'plan', project: String(project), id: ID });
      }
      return { view: view(project), report };
    },
    // Mock only, for the visual check: a camera agent's proposal beside the main line.
    mock_camera_agent: ({ project }: Args) => {
      const plan = ensure(project);
      const base = layoutOf(project).stops;
      const pick = (id: string) => base.find(s => s.id === id);
      const lantern: LayoutStop = {
        id: 'lantern',
        name: 'Lantern close-up',
        position: [4.4, 1.2, 2.9],
        target: [3.6, 0.9, 1.2],
        fov: 32,
        models: null,
      };
      const order = ['arrival', 'keystone', 'lantern', 'veil-gate', 'crystals', 'pullback'];
      const stops = order.flatMap(id => (id === 'lantern' ? [lantern] : [pick(id) ?? []].flat()));
      const line = lineOf(stops, 0, Math.max(...plan.nodes.map(n => n.y), 0) + 340);
      const by = 'Camera agent';
      const next: PlanFile = {
        ...plan,
        nodes: [...plan.nodes, ...line.nodes.map(n => ({ ...n, by }))],
        edges: [...plan.edges, ...line.edges.map(e => ({ ...e, by }))],
        edited: clock.now().toISOString(),
      };
      deps.put(project, ID, next);
      deps.logs.append(project, ID, {
        by,
        run: 'r-camera',
        ops: diffPlans(plan, next),
        summary: `Proposed a camera order (${stops.length} stops)`,
        mark: 'add',
      });
      deps.emit?.({ name: 'plan', project: String(project), id: ID });
      return null;
    },
    // Mock only, for the visual check: layout.json's stops change outside the plan.
    mock_layout_outside: ({ project }: Args) => {
      const layout = layoutOf(project);
      const [first, ...rest] = layout.stops;
      if (!first) return null;
      const stops = [...rest.slice(0, 1), first, ...rest.slice(1)].map(s =>
        s.id === 'keystone' ? { ...s, fov: 50 } : s
      );
      deps.layout?.save(String(project), { ...layout, stops });
      deps.emit?.({ name: 'layout', project: String(project) });
      return null;
    },
  };
}
