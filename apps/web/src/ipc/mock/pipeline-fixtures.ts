/**
 * Mock pipeline data: the Floating diorama steps (the same shape the Rust
 * template writes), a short run history and realistic Blender output.
 */
import type {
  CompressPreset,
  LogEntry,
  ModelFile,
  PipelineDef,
  PipelineView,
  RunRecord,
  StarterState,
  StarterStatus,
  StepDef,
  StepRecord,
} from '../client';
import { BAKE_DEFAULT } from './bake';

const HEADLESS = ['-b', '--factory-startup', '--python-exit-code', '1'];

function blender(
  id: string,
  name: string,
  script: string,
  blend: string,
  extra: string[],
  inputs: string[],
  outputs: string[],
  dependsOn: string[]
): StepDef {
  return {
    id,
    name,
    kind: 'blender',
    command: null,
    script,
    args: [...HEADLESS, blend, '--python', '{script}', '--', ...extra],
    inputs,
    outputs,
    dependsOn,
    compress: null,
  };
}

export const CHARACTER_LOCK =
  'Elsheria stays on medium. High meshopt quantisation makes her mesh spin at the camera stops.';

/** The Ship starter's args, as the template writes them. */
const SHIP_ARGS =
  '{script} --models web/dist --posters web/posters --site {siteRepo} --index {shipIndex} --layout {project}/layout.json --materials {project}/materials.json';

export function dioramaPipeline(): PipelineDef {
  return {
    version: 1,
    root: null,
    steps: [
      blender(
        'build',
        'Build scene',
        'tools/build_scene.py',
        'blender/scene.blend',
        ['--out', 'blender/scene_built.blend'],
        ['blender/scene.blend', 'models/*.glb'],
        ['blender/scene_built.blend'],
        []
      ),
      blender(
        'bake',
        'Bake light',
        'tools/bake_light.py',
        'blender/scene_built.blend',
        ['--out-dir', 'blender/bake', '--size', '1024', '--samples', '32', '--denoise'],
        ['blender/scene_built.blend'],
        ['blender/bake/*.png', 'blender/scene_baked.blend'],
        ['build']
      ),
      blender(
        'export',
        'Export GLB',
        'tools/export_web.py',
        'blender/scene_baked.blend',
        ['--out', 'web/raw'],
        ['blender/scene_baked.blend'],
        ['web/raw/*.glb'],
        ['bake']
      ),
      {
        id: 'compress',
        name: 'Compress',
        kind: 'gltf-transform',
        command: null,
        script: null,
        args: [],
        inputs: ['web/raw/*.glb'],
        outputs: ['web/dist/*.glb'],
        dependsOn: ['export'],
        compress: { from: 'web/raw', to: 'web/dist' },
      },
      blender(
        'posters',
        'Capture posters',
        'tools/render_stops.py',
        'blender/scene_baked.blend',
        ['--out', 'web/posters'],
        ['blender/scene_baked.blend'],
        ['web/posters/*.png'],
        ['compress']
      ),
      {
        id: 'ship',
        name: 'Ship',
        kind: 'shell',
        command: 'node',
        script: 'tools/ship.mjs',
        args: SHIP_ARGS.split(' '),
        inputs: ['web/dist/*.glb', 'web/posters/*.png'],
        outputs: [],
        dependsOn: ['compress', 'posters'],
        compress: null,
      },
    ],
    presets: presets(),
  };
}

function presets(): CompressPreset[] {
  return [
    {
      model: 'crystal_lantern',
      label: 'Crystal lantern',
      textureSize: 1024,
      codec: 'etc1s',
      meshopt: 'medium',
      lock: null,
    },
    {
      model: 'veil_gate',
      label: 'Veil gate',
      textureSize: 1024,
      codec: 'etc1s',
      meshopt: 'high',
      lock: null,
    },
    {
      model: 'elsheria',
      label: 'Elsheria',
      textureSize: 2048,
      codec: 'etc1s',
      meshopt: 'medium',
      lock: CHARACTER_LOCK,
    },
  ];
}

const MIN = 60_000;

function step(
  id: string,
  name: string,
  state: StepRecord['state'],
  durationMs: number | null,
  message: string | null = null
): StepRecord {
  return {
    id,
    name,
    state,
    startedAtMs: null,
    durationMs,
    exitCode: state === 'failed' ? 1 : state === 'done' ? 0 : null,
    message,
  };
}

/** Run 23 done, run 24 failed at Capture posters. Newest first. */
export function runHistory(now: number): RunRecord[] {
  const r23: RunRecord = {
    id: 'frostveil-run-23',
    number: 23,
    project: 'frostveil',
    from: null,
    force: false,
    state: 'done',
    queuedAtMs: now - 95 * MIN,
    startedAtMs: now - 95 * MIN,
    finishedAtMs: now - 94 * MIN,
    steps: [
      step('build', 'Build scene', 'cached', 12),
      step('bake', 'Bake light', 'cached', 9),
      step('export', 'Export GLB', 'done', 18_400),
      step('compress', 'Compress', 'done', 6_100),
      step('posters', 'Capture posters', 'done', 21_800),
      step('ship', 'Ship', 'done', 900),
    ],
    message: null,
  };
  const r24: RunRecord = {
    id: 'frostveil-run-24',
    number: 24,
    project: 'frostveil',
    from: null,
    force: false,
    state: 'failed',
    queuedAtMs: now - 12 * MIN,
    startedAtMs: now - 12 * MIN,
    finishedAtMs: now - 11 * MIN,
    steps: [
      step('build', 'Build scene', 'cached', 10),
      step('bake', 'Bake light', 'cached', 8),
      step('export', 'Export GLB', 'done', 17_900),
      step('compress', 'Compress', 'done', 6_400),
      step('posters', 'Capture posters', 'failed', 3_200, 'blender exited with code 1.'),
      step('ship', 'Ship', 'skipped', null, 'Not run, an earlier step failed.'),
    ],
    message:
      'Capture posters failed. Camera "poster" is missing in scene_baked.blend. Add it, then run again from this step.',
  };
  return [r24, r23];
}

/** Blender's own lines for a step, as the real runner streams them. */
export function blenderLines(
  stepId: string
): { text: string; level?: LogEntry['level']; stream?: LogEntry['stream'] }[] {
  switch (stepId) {
    case 'export':
      return [
        { text: 'Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17)' },
        { text: 'Read blend: "blender/scene_baked.blend"' },
        { text: '15:02:22 | INFO: Starting glTF 2.0 export' },
        { text: '[export_web] 17 objects to glTF 2.0' },
        { text: '15:02:22 | WARNING: More than one shader node tex image used for a texture.' },
        { text: '15:02:23 | INFO: Extracting primitive: platform_ground' },
        { text: '15:02:24 | INFO: Extracting primitive: observatory_arch' },
        { text: 'wrote web/raw/veil_platform.glb 4.6 MB' },
        { text: 'wrote web/raw/observatory_arch.glb 2.4 MB' },
        { text: 'wrote web/raw/crystal_lantern.glb 1.1 MB' },
        { text: 'exporting elsheria.glb, 2 clips' },
        { text: '15:02:31 | INFO: Animation spellblade_idle: 62 frames' },
        { text: '15:02:33 | INFO: Animation draw_sword_1: 16 frames' },
        { text: 'wrote web/raw/elsheria.glb 6.2 MB' },
        { text: 'Blender quit' },
      ];
    case 'compress':
      return [
        { text: 'Draco compression is not available, using meshopt' },
        { text: 'veil_platform textures to ETC1S at 2048 px', stream: 'system' },
        { text: 'info: veil_platform.glb (4.6 MB) → veil_platform.tex.glb (2.1 MB)' },
        { text: 'info: prune: Removed types... Accessor (5), Texture (1)' },
        { text: 'veil_platform geometry with meshopt high', stream: 'system' },
        { text: 'info: veil_platform.tex.glb (2.1 MB) → veil_platform.glb (1.1 MB)' },
        { text: 'elsheria geometry with meshopt medium', stream: 'system' },
        { text: 'info: elsheria.tex.glb (2.8 MB) → elsheria.glb (1.1 MB)' },
      ];
    case 'posters':
      return [
        { text: 'Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17)' },
        { text: 'Read blend: "blender/scene_baked.blend"' },
        { text: '[render_stops] 4 camera stops' },
        { text: 'Traceback (most recent call last):', level: 'error', stream: 'stderr' },
        {
          text: '  File "tools/render_stops.py", line 41, in <module>',
          level: 'error',
          stream: 'stderr',
        },
        {
          text: 'KeyError: \'bpy_prop_collection[key]: key "poster" not found\'',
          level: 'error',
          stream: 'stderr',
        },
      ];
    default:
      return [{ text: 'Blender 4.3.2 (hash 32f5fdce0a0a built 2024-12-17)' }];
  }
}

/** The stored log of run 24. */
export function run24Log(now: number): LogEntry[] {
  const at = now - 12 * MIN;
  const entry = (
    i: number,
    step: string | null,
    text: string,
    level: LogEntry['level'] = 'info',
    stream: LogEntry['stream'] = 'stdout'
  ): LogEntry => ({
    runId: 'frostveil-run-24',
    step,
    stream,
    level,
    text,
    atMs: at + i * 900,
  });
  let i = 0;
  const out: LogEntry[] = [
    entry(i++, null, 'Run 24 · 6 of 6 steps', 'info', 'system'),
    entry(i++, 'build', 'Build scene inputs unchanged, cache hit', 'ok', 'system'),
    entry(i++, 'bake', 'Bake light inputs unchanged, cache hit', 'ok', 'system'),
  ];
  for (const l of blenderLines('export')) out.push(entry(i++, 'export', l.text, l.level, l.stream));
  out.push(entry(i++, 'export', 'Export GLB done in 17.9s', 'ok', 'system'));
  for (const l of blenderLines('compress'))
    out.push(entry(i++, 'compress', l.text, l.level, l.stream));
  out.push(entry(i++, 'compress', 'Compress done in 6.4s', 'ok', 'system'));
  for (const l of blenderLines('posters'))
    out.push(entry(i++, 'posters', l.text, l.level ?? 'info', l.stream));
  out.push(
    entry(
      i++,
      'posters',
      'Capture posters failed after 3.2s: blender exited with code 1.',
      'error',
      'system'
    )
  );
  out.push(
    entry(
      i,
      null,
      'Run 24 failed: Capture posters failed. Camera "poster" is missing.',
      'error',
      'system'
    )
  );
  return out;
}

function model(
  stem: string,
  assetName: string | null,
  version: number | null,
  rawBytes: number,
  shippedBytes: number
): ModelFile {
  return { model: stem, assetName, version, rawBytes, shippedBytes };
}

export function pipelineView(
  project: string,
  root: string,
  runs: RunRecord[],
  log: LogEntry[]
): PipelineView {
  return {
    project,
    root,
    def: dioramaPipeline(),
    issues: [],
    runs,
    active: runs.find(r => r.state === 'running')?.id ?? null,
    queued: [],
    log,
    artifacts: runs.length
      ? [
          { step: 'export', path: 'web/raw/veil_platform.glb', bytes: 4.6e6 },
          { step: 'compress', path: 'web/dist/veil_platform.glb', bytes: 1.1e6 },
        ]
      : [],
    models: runs.length
      ? [
          model('veil_platform', 'Veil platform', 7, 4.6e6, 1.1e6),
          model('observatory_arch', 'Observatory arch', 3, 2.4e6, 0.9e6),
          model('crystal_lantern', 'Crystal lantern', 2, 1.1e6, 0.4e6),
          model('elsheria', 'Elsheria', 2, 6.2e6, 1.07e6),
        ]
      : [],
    defaults: { textureSize: 2048, codec: 'etc1s', meshopt: 'high' },
    starters: currentStarters(),
    firstView: { stop: null, custom: false, models: [], bytes: 0, targetMb: 3 },
    bake: { settings: BAKE_DEFAULT, canDenoise: true, script: 'current', skipped: [] },
  };
}

/** The diorama starters, as a project made today has them. */
export const DIORAMA_STARTERS = [
  'tools/build_scene.py',
  'tools/bake_light.py',
  'tools/export_web.py',
  'tools/render_stops.py',
  'tools/ship.mjs',
] as const;

export function currentStarters(): StarterStatus[] {
  return DIORAMA_STARTERS.map(path => ({ path, state: 'current', backup: null }));
}

/**
 * Frostveil was made before the starters grew up: three untouched old
 * copies and one the user edited.
 */
export function oldStarters(): StarterStatus[] {
  const state: Record<string, StarterState> = {
    'tools/build_scene.py': 'outdated',
    'tools/bake_light.py': 'outdated',
    'tools/export_web.py': 'edited',
    'tools/render_stops.py': 'outdated',
  };
  return DIORAMA_STARTERS.map(path => ({ path, state: state[path] ?? 'current', backup: null }));
}
