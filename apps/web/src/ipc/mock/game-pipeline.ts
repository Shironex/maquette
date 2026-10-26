/** The game pipeline in mock mode, as the core's game template writes it, and its runs. */
import type { GameSettings, PipelineDef, RunRecord, StepDef, StepRecord } from '../client';

const HEADLESS = ['-b', '--factory-startup', '--python-exit-code', '1'];
const MIN = 60_000;

function step(def: Partial<StepDef> & Pick<StepDef, 'id' | 'name' | 'kind'>): StepDef {
  return {
    command: null,
    script: null,
    args: [],
    inputs: [],
    outputs: [],
    dependsOn: [],
    compress: null,
    ...def,
  };
}

/** The game pipeline, as the core's game template writes it. */
export function gamePipeline(settings: GameSettings): PipelineDef {
  const index = ['--index', 'web/game-index.json'];
  const built = 'blender/game_built.blend';
  const from = settings.bake ? 'blender/game_baked.blend' : built;
  const steps: StepDef[] = [
    step({
      id: 'build',
      name: 'Build models',
      kind: 'blender',
      script: 'tools/build_models.py',
      args: [...HEADLESS, '--python', '{script}', '--', ...index, '--out', built],
      inputs: ['web/game-index.json', 'prepare.json', 'models/*.glb', 'models/*.obj'],
      outputs: [built],
    }),
  ];
  if (settings.bake) {
    steps.push(
      step({
        id: 'bake',
        name: 'Bake light',
        kind: 'blender',
        script: 'tools/bake_light.py',
        args: [
          ...HEADLESS,
          built,
          '--python',
          '{script}',
          '--',
          '--out',
          'blender/game_baked.blend',
        ],
        inputs: [built],
        outputs: ['blender/game_baked.blend'],
        dependsOn: ['build'],
      })
    );
  }
  const cap = String(settings.export.textureCap);
  steps.push(
    step({
      id: 'export',
      name: settings.format === 'obj' ? 'Export OBJ' : 'Export GLB',
      kind: 'blender',
      script: 'tools/export_game.py',
      args: [...HEADLESS, from, '--python', '{script}', '--', ...index, '--out', 'web/raw'],
      inputs: [from],
      outputs: ['web/raw/export.json'],
      dependsOn: [settings.bake ? 'bake' : 'build'],
    }),
    step({
      id: 'game',
      name: 'Game export',
      kind: 'shell',
      command: 'node',
      script: 'tools/game_export.mjs',
      args: ['{script}', ...index, '--raw', 'web/raw', '--out', 'web/game', '--cap', cap],
      inputs: ['web/game-index.json', 'web/raw/**/*'],
      outputs: ['web/game-export.json'],
      dependsOn: ['export'],
    }),
    step({
      id: 'ship',
      name: 'Ship',
      kind: 'shell',
      command: 'node',
      script: 'tools/ship_game.mjs',
      args: [
        '{script}',
        '--from',
        'web/game',
        '--game',
        '{siteRepo}',
        '--models',
        settings.modelsDir,
      ],
      dependsOn: ['game'],
    })
  );
  return { version: 1, root: null, steps, presets: [] };
}

function record(
  id: string,
  name: string,
  state: StepRecord['state'],
  ms: number | null
): StepRecord {
  return { id, name, state, startedAtMs: null, durationMs: ms, exitCode: null, message: null };
}

/** Run 6 shipped; run 7 waits for approval before Ship. */
export function gameRuns(project: string, def: PipelineDef, now: number): RunRecord[] {
  const steps = (last: StepRecord['state']) =>
    def.steps.map((s, i) =>
      record(s.id, s.name, i === def.steps.length - 1 ? last : 'done', i === 3 ? null : 4_200)
    );
  const run = (number: number, state: RunRecord['state'], ago: number): RunRecord => ({
    id: `${project}-run-${number}`,
    number,
    project,
    from: null,
    force: false,
    state,
    queuedAtMs: now - ago,
    startedAtMs: now - ago,
    finishedAtMs: state === 'running' ? null : now - ago + MIN,
    steps: steps(state === 'running' ? 'waiting' : 'done'),
    message: null,
  });
  return [run(7, 'running', 2 * MIN), run(6, 'done', 60 * MIN)];
}
