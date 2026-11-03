import { describe, expect, it } from 'vitest';
import type { LayoutStop, PlanDoc, PlanFile, SceneLayout } from '../client';
import { createCameraHandlers } from './create-camera-handlers';
import type { createFakeLayout } from './layout';
import { createMockLogs } from './plan-log';

const stop = (id: string, x: number): LayoutStop => ({
  id,
  name: id,
  position: [x, 1, 3],
  target: [0, 0.5, 0],
  fov: 35,
  models: null,
});

/** The camera handlers over one project whose layout.json the test changes. */
function setup() {
  let layout = { stops: [stop('porch', 0), stop('moon', 1)] } as unknown as SceneLayout;
  const plans = new Map<string, PlanFile>();
  const fake = {
    view: () => ({ layout }),
    save: (_: string, next: SceneLayout) => {
      layout = next;
    },
  } as unknown as ReturnType<typeof createFakeLayout>;
  const handlers = createCameraHandlers({
    get: (_, id) => plans.get(id),
    put: (_, id, plan) => void plans.set(id, plan),
    doc: (_, id, plan) => ({ id, file: `plans/${id}.plan.json`, plan, images: {} }) as PlanDoc,
    logs: createMockLogs(),
    layout: fake,
    projectDir: () => '/mock/moon-island',
  });
  const move = (stops: LayoutStop[]) => {
    layout = { ...layout, stops };
  };
  return { handlers, move };
}

describe('mock camera plan', () => {
  it('refuses a rebase when layout.json changed since the comparison shown', () => {
    const { handlers, move } = setup();
    handlers.plan_camera({ project: 'moon-island' });
    move([stop('moon', 1), stop('porch', 0)]);
    const shown = handlers.plan_camera_rebase({
      project: 'moon-island',
      mode: 'rebase',
      dry: true,
    });
    expect(shown.view.layoutKey).toBe(shown.report.layoutKey);
    move([stop('moon', 1), stop('lantern', 2), stop('porch', 0)]);
    expect(() =>
      handlers.plan_camera_rebase({
        project: 'moon-island',
        mode: 'start-over',
        dry: false,
        layoutKey: shown.report.layoutKey,
      })
    ).toThrow(/Review again/);
    const fresh = handlers.plan_camera_rebase({
      project: 'moon-island',
      mode: 'rebase',
      dry: true,
    });
    const done = handlers.plan_camera_rebase({
      project: 'moon-island',
      mode: 'rebase',
      dry: false,
      layoutKey: fresh.report.layoutKey,
    });
    expect(done.report.change).not.toBeNull();
    expect(done.view.stale).toBe(false);
  });
});
