import { describe, expect, it } from 'vitest';
import type {
  AcceptResult,
  Bootstrap,
  LabView,
  MovedDirection,
  ProjectSummary,
  WorkspaceSnapshot,
  InspectView,
  LayoutView,
  PipelineEvent,
  PipelineView,
  ProjectManifest,
  ProviderJob,
  RunRecord,
  SceneMaterials,
  ScenePrep,
  SendOptions,
} from '../client';
import { presetRows, presetsToSave } from '@/state/presets';
import { createBackend } from './backend';

function backend(scenario = 'idle') {
  const events: PipelineEvent[] = [];
  let inboxEvents = 0;
  const handle = createBackend({
    scenario,
    probeDelay: 0,
    pace: 0,
    emit: e => (e.name === 'pipeline' ? events.push(e.payload) : inboxEvents++),
  });
  return { handle, events, inboxCount: () => inboxEvents };
}

async function finished(handle: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>) {
  for (let i = 0; i < 200; i++) {
    const view = (await handle('pipeline_get', { project: 'frostveil' })) as PipelineView;
    if (!view.active) return view;
    await new Promise(r => setTimeout(r, 5));
  }
  throw new Error('run did not finish');
}

describe('mock backend', () => {
  it('inspects the scene with the presets the pipeline stores', async () => {
    const { handle } = backend('configured');
    const view = (await handle('inspect_project', { project: 'frostveil' })) as InspectView;
    expect(view.comparisons[0]!.sides.map(s => s.label)).toEqual(['v6', 'v7']);
    expect(view.stops.stops).toHaveLength(5);
    const character = view.models.find(m => m.model === 'elsheria')!;
    expect(character.preset.lock).toMatch(/medium/);
    await handle('pipeline_save_presets', {
      project: 'frostveil',
      presets: [
        ...((await handle('pipeline_get', { project: 'frostveil' })) as PipelineView).def.presets,
        {
          model: 'ice_crystals',
          label: 'Crystals',
          textureSize: 512,
          codec: 'uastc',
          meshopt: 'low',
          lock: null,
        },
      ],
    });
    const after = (await handle('inspect_project', { project: 'frostveil' })) as InspectView;
    const shrubs = after.models.find(m => m.model === 'ice_crystals')!;
    expect(shrubs.label).toBe('Crystals');
    expect(shrubs.preset).toMatchObject({ codec: 'uastc', meshopt: 'low', hasRow: true });
  });

  it('runs the pipeline, fails at posters once, then passes from there', async () => {
    const { handle, events } = backend('configured');
    const run = (await handle('pipeline_run', {
      project: 'frostveil',
      ask: { from: null, force: false },
    })) as RunRecord;
    expect(run.number).toBe(25);
    let view = await finished(handle);
    expect(view.runs[0]!.state).toBe('failed');
    expect(view.runs[0]!.steps.find(s => s.id === 'build')!.state).toBe('cached');
    expect(view.runs[0]!.steps.find(s => s.id === 'ship')!.state).toBe('skipped');
    expect(events.some(e => e.type === 'log' && e.entry.level === 'error')).toBe(true);

    await handle('pipeline_run', { project: 'frostveil', ask: { from: 'posters' } });
    view = await finished(handle);
    expect(view.runs[0]!.state).toBe('done');
    expect(view.runs[0]!.steps.find(s => s.id === 'export')!.state).toBe('skipped');
  });

  it('passes every step in the idle scenario', async () => {
    const { handle } = backend('idle');
    await handle('pipeline_run', { project: 'frostveil', ask: { force: true } });
    const view = await finished(handle);
    expect(view.runs[0]!).toMatchObject({ number: 24, state: 'done' });
    expect(view.runs[0]!.steps.every(s => s.state === 'done')).toBe(true);
  });

  it('leaves Ship out of a build only run', async () => {
    const { handle } = backend('idle');
    await handle('pipeline_run', { project: 'frostveil', ask: { force: true, buildOnly: true } });
    const view = await finished(handle);
    const ship = view.runs[0]!.steps.find(s => s.id === 'ship')!;
    expect(view.runs[0]!).toMatchObject({ state: 'done', buildOnly: true });
    expect(ship).toMatchObject({ state: 'skipped', message: 'Left out: this run builds only.' });
  });

  it('refuses edits to a locked preset row', async () => {
    const { handle } = backend();
    const view = (await handle('pipeline_get', { project: 'frostveil' })) as PipelineView;
    const presets = view.def.presets.map(p => (p.lock ? { ...p, meshopt: 'high' } : p));
    await expect(
      handle('pipeline_save_presets', { project: 'frostveil', presets })
    ).rejects.toMatchObject({
      code: 'invalid',
    });
  });

  it('adds and moves assets', async () => {
    const { handle } = backend();
    let project = (await handle('asset_add', {
      project: 'frostveil',
      asset: { name: 'Paper lamp', kind: 'prop', stage: 'concept' },
    })) as ProjectManifest;
    expect(project.assets!.find(a => a.id === 'paper-lamp')).toMatchObject({ stage: 'concept' });
    project = (await handle('asset_move', {
      project: 'frostveil',
      id: 'paper-lamp',
      stage: 'refs',
    })) as ProjectManifest;
    expect(project.assets!.find(a => a.id === 'paper-lamp')!.stage).toBe('refs');
  });

  it('renames an asset and keeps its id', async () => {
    const { handle } = backend();
    await handle('asset_add', {
      project: 'frostveil',
      asset: { name: 'Paper lamp', kind: 'prop', stage: 'concept' },
    });
    await handle('asset_add', {
      project: 'frostveil',
      asset: { name: 'Stone bench', kind: 'prop', stage: 'concept' },
    });
    const project = (await handle('asset_rename', {
      project: 'frostveil',
      id: 'paper-lamp',
      name: ' Hanging lamp ',
    })) as ProjectManifest;
    expect(project.assets!.find(a => a.id === 'paper-lamp')!.name).toBe('Hanging lamp');
    await expect(
      handle('asset_rename', { project: 'frostveil', id: 'paper-lamp', name: 'stone bench' })
    ).rejects.toMatchObject({ code: 'invalid' });
    await expect(
      handle('asset_rename', { project: 'frostveil', id: 'nope', name: 'x' })
    ).rejects.toMatchObject({ code: 'not-found' });
  });

  it('accepts an inbox model into the project', async () => {
    const { handle, inboxCount } = backend('configured');
    const result = (await handle('inbox_accept', {
      request: { item: 'veil_gate.glb', project: 'frostveil', asset: null },
    })) as AcceptResult;
    expect(result.version.file).toBe('models/veil_gate_v1.glb');
    expect(result.asset).toMatchObject({ stage: 'blender', status: 'review' });
    expect(result.project.assets!.some(a => a.id === result.asset.id)).toBe(true);
    expect(inboxCount()).toBe(1);
    const snapshot = (await handle('inbox_list')) as { items: { id: string }[] };
    expect(snapshot.items.map(i => i.id)).not.toContain('veil_gate.glb');
  });

  it('adds a version to the asset a model was sent for', async () => {
    const { handle } = backend('configured');
    const result = (await handle('inbox_accept', {
      request: { item: 'veil_gate.glb', project: 'frostveil', asset: 'veil-gate' },
    })) as AcceptResult;
    expect(result.asset.id).toBe('veil-gate');
    expect(result.version.file).toMatch(/^models\/veil_gate_v\d+\.glb$/);
  });

  it('saves the real size and fix-ups set before Accept, or refuses them before the move', async () => {
    const { handle } = backend('configured');
    await expect(
      handle('inbox_accept', {
        request: {
          item: 'veil_gate.glb',
          project: 'frostveil',
          asset: null,
          prep: { size: null, fixups: [{ op: 'decimate', triangles: 5 }] },
        },
      })
    ).rejects.toMatchObject({ code: 'invalid' });
    const listed = (await handle('inbox_list')) as { items: { id: string }[] };
    expect(listed.items.map(i => i.id)).toContain('veil_gate.glb');

    const prep = {
      size: { axis: 'height', value: 6, unit: 'm' },
      fixups: [{ op: 'origin-bottom' }, { op: 'merge', byMaterial: false }],
    };
    const result = (await handle('inbox_accept', {
      request: { item: 'veil_gate.glb', project: 'frostveil', asset: null, prep },
    })) as AcceptResult;
    const saved = (await handle('prepare_get', { project: 'frostveil' })) as ScenePrep;
    expect(saved.models[result.asset.id]?.fixups.map(f => f.op)).toEqual([
      'merge',
      'origin-bottom',
    ]);
    const layout = (await handle('layout_get', { project: 'frostveil' })) as LayoutView;
    const placed = layout.models.find(m => m.asset === result.asset.id)!;
    expect(placed.max?.[1]).toBeCloseTo(6);
    expect(placed.min?.[1]).toBeCloseTo(0);
  });

  it('carries the inbox thumbnail onto the accepted version', async () => {
    const aliases: [string, string][] = [];
    const handle = createBackend({
      scenario: 'configured',
      probeDelay: 0,
      pace: 0,
      files: { alias: (path, from) => aliases.push([path, from]), store: () => {} },
    });
    const result = (await handle('inbox_accept', {
      request: { item: 'veil_gate.glb', project: 'frostveil', asset: null },
    })) as AcceptResult;
    expect(result.asset.thumbnail).toBe('thumbs/veil_gate_v1.png');
    expect(aliases).toHaveLength(1);
    expect(aliases[0]![0]).toMatch(/projects\/frostveil\/thumbs\/veil_gate_v1\.png$/);
    expect(aliases[0]![1]).toMatch(/cache\/thumbs\/veil-gate\.webp$/);
  });

  it('stores a snapshot thumbnail for an inbox model', async () => {
    const stored: string[] = [];
    const handle = createBackend({
      scenario: 'configured',
      probeDelay: 0,
      pace: 0,
      files: { alias: () => {}, store: path => stored.push(path) },
    });
    const snapshot = (await handle('inbox_list')) as { items: { file: string; id: string }[] };
    const file = snapshot.items.find(i => i.id === 'elsheria.glb')!.file;
    const path = (await handle('thumbnail_store', { file, png: [137, 80, 78, 71] })) as string;
    expect(stored).toEqual([path]);
    const after = (await handle('inbox_list')) as {
      items: { id: string; thumbnail: string | null; thumbnailState: string }[];
    };
    expect(after.items.find(i => i.id === 'elsheria.glb')).toMatchObject({
      thumbnail: path,
      thumbnailState: 'ready',
    });
  });

  it('sends by hand, takes the model back and refuses a send without a key', async () => {
    const { handle } = backend('configured');
    const options = (await handle('provider_send_options', {
      project: 'frostveil',
      asset: 'guardian-statues',
    })) as SendOptions;
    expect(options.suggested).toEqual(['refs/guardian_statues_front.png']);
    expect(options.keys.find(k => k.service === 'meshy')?.saved).toBe(false);
    const request = {
      project: 'frostveil',
      asset: 'guardian-statues',
      provider: 'by-hand',
      service: 'tripo',
      variant: 'prop',
      images: options.suggested,
      tripo: null,
      meshy: null,
    };
    const job = (await handle('provider_send', { request })) as ProviderJob;
    expect(job.state).toBe('waiting');
    expect(job.card?.exportName).toBe(
      `maq_guardian-statues_v${job.id.slice('guardian-statues-'.length)}_tripo`
    );
    expect(job.card?.returnName).toBe(`${job.card?.exportName}.glb`);
    expect(job.card?.credits).toBe(30);
    await expect(
      handle('provider_hand_back', { id: job.id, file: '/tmp/a.png' })
    ).rejects.toMatchObject({
      code: 'invalid',
    });
    const back = (await handle('provider_hand_back', {
      id: job.id,
      file: '/tmp/a.glb',
    })) as ProviderJob;
    expect(back.state).toBe('inbox');
    await expect(
      handle('provider_send', { request: { ...request, provider: 'meshy', service: 'meshy' } })
    ).rejects.toMatchObject({ code: 'invalid' });
    const tripo = (await handle('provider_cancel', { id: 'veil-gate-1' }).catch(e => e)) as {
      message: string;
    };
    expect(tripo.message).toContain('Tripo cannot cancel');
  });

  it('lists three projects, switches between them and keeps the switch on a settings save', async () => {
    const { handle } = backend('configured');
    const list = (await handle('project_list')) as ProjectSummary[];
    expect(list.map(p => [p.name, p.template])).toEqual([
      ['Frostveil', 'floating-diorama'],
      ['Frost cathedral', 'room'],
      ['Elsheria showcase', 'character-showcase'],
    ]);
    expect(list[0]!.thumb).toBe('thumbs/veil-platform.webp');
    const snap = (await handle('project_switch', { id: 'frost-cathedral' })) as WorkspaceSnapshot;
    expect(snap.project?.id).toBe('frost-cathedral');
    await handle('workspace_save_config', {
      config: { ...snap.config, currentProject: 'frostveil' },
    });
    const boot = (await handle('app_bootstrap')) as Bootstrap;
    expect(boot.workspace?.project?.id).toBe('frost-cathedral');
    await expect(handle('project_switch', { id: 'nope' })).rejects.toMatchObject({
      code: 'not-found',
    });
    const pipeline = (await handle('pipeline_get', { project: 'frost-cathedral' })) as PipelineView;
    expect(pipeline.runs).toEqual([]);
  });

  it('gives the landing studio card its ref and moves the direction to a new project', async () => {
    const { handle } = backend('configured');
    const boot = (await handle('app_bootstrap')) as Bootstrap;
    const card = boot.workspace?.project?.assets?.find(a => a.id === 'academy-landing');
    expect(card?.thumbnail).toBe('refs/academy-landing_04.png');
    await handle('project_create', {
      project: {
        name: 'Maquette landing',
        template: 'floating-diorama',
        siteRepo: null,
        sourceFolder: null,
      },
    });
    const moved = (await handle('lab_move_direction', {
      project: 'frostveil',
      direction: 'academy-landing',
      to: 'maquette-landing',
    })) as MovedDirection;
    expect(moved.direction.images[3]).toMatchObject({ picked: true, starred: true });
    expect(moved.asset?.thumbnail).toBe('refs/academy-landing_04.png');
    expect((moved.from.assets ?? []).some(a => a.id === 'academy-landing')).toBe(false);
    expect((moved.to.assets ?? []).map(a => a.id)).toEqual(['academy-landing']);
    const moon = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    expect(moon.directions.some(d => d.id === 'academy-landing')).toBe(false);
    const landing = (await handle('lab_get', { project: 'maquette-landing' })) as LabView;
    expect(landing.directions.map(d => d.name)).toEqual(['Academy landing']);
  });
});

describe('mock starter scripts', () => {
  it('updates untouched old starters and replaces an edited one only when asked', async () => {
    const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
    const call = <T>(cmd: string, args: Record<string, unknown> = {}) => handle(cmd, args) as T;
    type View = { starters: { path: string; state: string; backup: string | null }[] };
    const before = await call<View>('pipeline_get', { project: 'frostveil' });
    expect(before.starters.filter(s => s.state === 'outdated')).toHaveLength(3);
    const all = before.starters.map(s => s.path);
    const kept = await call<View>('pipeline_update_starters', {
      project: 'frostveil',
      scripts: all,
      replace: false,
    });
    expect(kept.starters.map(s => s.state)).toEqual([
      'current',
      'current',
      'edited',
      'current',
      'current',
    ]);
    const replaced = await call<View>('pipeline_update_starters', {
      project: 'frostveil',
      scripts: ['tools/export_web.py'],
      replace: true,
    });
    expect(replaced.starters.find(s => s.path === 'tools/export_web.py')).toEqual({
      path: 'tools/export_web.py',
      state: 'current',
      backup: 'tools/export_web.py.bak',
    });
    expect(
      (await call<View>('pipeline_get', { project: 'frost-cathedral' })).starters.every(
        s => s.state === 'current'
      )
    ).toBe(true);
  });
});

describe('mock presets', () => {
  it('gives only projects with a character the character row', async () => {
    const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
    const presetsOf = async (project: string) =>
      ((await handle('pipeline_get', { project })) as PipelineView).def.presets.map(p => p.model);
    expect(await presetsOf('frostveil')).toContain('elsheria');
    expect(await presetsOf('frost-cathedral')).toEqual([]);
  });

  it('takes the rows the panel saves back, the locked one included', async () => {
    const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
    const view = (await handle('pipeline_get', { project: 'frostveil' })) as PipelineView;
    const rows = presetsToSave(presetRows(view));
    const saved = (await handle('pipeline_save_presets', {
      project: 'frostveil',
      presets: rows,
    })) as PipelineView;
    expect(saved.def.presets).toEqual(rows);
  });

  it('copies an asset under a free name and counts what the first stop sees', async () => {
    const { handle } = backend('configured');
    const first = (await handle('layout_get', { project: 'frostveil' })) as LayoutView;
    expect(first.firstView.stop).toBe('Arrival');
    expect(first.firstView.models.length).toBeGreaterThan(0);
    expect(first.firstView.targetMb).toBe(3);
    const copy = async () =>
      (await handle('asset_copy_to_project', {
        project: 'frostveil',
        id: 'elsheria',
        to: 'frost-cathedral',
      })) as { asset: { id: string; name: string; versions: { file: string }[] } };
    const one = await copy();
    const two = await copy();
    expect(one.asset.id).toBe('elsheria');
    expect(two.asset).toMatchObject({ id: 'elsheria-2', name: 'Elsheria 2' });
    expect(two.asset.versions[0]!.file).toMatch(/^models\/elsheria_2_v\d+\.glb$/);
    const source = (await handle('layout_get', { project: 'frostveil' })) as LayoutView;
    expect(source.models.some(m => m.asset === 'elsheria')).toBe(true);
    const target = (await handle('layout_get', { project: 'frost-cathedral' })) as LayoutView;
    expect(target.models.map(m => m.asset)).toEqual(
      expect.arrayContaining(['elsheria', 'elsheria-2'])
    );
  });

  it('keeps material tweaks per project, tidied, and refuses a locked map codec change', async () => {
    const { handle } = backend('configured');
    const none = { tint: null, roughness: null, metalness: null, emissiveStrength: null };
    const stored = await handle('materials_save', {
      project: 'frostveil',
      looks: {
        'veil-gate': { all: { ...none, tint: '#FFC0CB' }, materials: { Leaves: none } },
        hut: { all: none, materials: {} },
      },
    });
    expect(stored).toEqual({
      version: 1,
      models: { 'veil-gate': { all: { ...none, tint: '#ffc0cb' }, materials: {} } },
    });
    // Another asset's look lands beside the stored one.
    const both = (await handle('materials_save', {
      project: 'frostveil',
      looks: { hut: { all: { ...none, metalness: 0.5 }, materials: {} } },
    })) as SceneMaterials;
    expect(Object.keys(both.models)).toEqual(['veil-gate', 'hut']);
    expect(await handle('materials_get', { project: 'frost-cathedral' })).toEqual({
      version: 1,
      models: {},
    });
    await expect(
      handle('materials_save', {
        project: 'frostveil',
        looks: {
          'veil-gate': {
            all: { ...none, roughness: { mode: 'multiply', value: 3 } },
            materials: {},
          },
        },
      })
    ).rejects.toMatchObject({ code: 'invalid' });

    const view = (await handle('pipeline_get', { project: 'frostveil' })) as PipelineView;
    const rows = view.def.presets.map(p =>
      p.lock ? { ...p, mapCodecs: { color: null, normal: 'etc1s', other: null } } : p
    );
    await expect(
      handle('pipeline_save_presets', { project: 'frostveil', presets: rows })
    ).rejects.toMatchObject({ code: 'invalid' });
  });
});
