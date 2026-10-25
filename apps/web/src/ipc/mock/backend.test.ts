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
    const view = (await handle('pipeline_get', { project: 'moon-island' })) as PipelineView;
    if (!view.active) return view;
    await new Promise(r => setTimeout(r, 5));
  }
  throw new Error('run did not finish');
}

describe('mock backend', () => {
  it('inspects the scene with the presets the pipeline stores', async () => {
    const { handle } = backend('configured');
    const view = (await handle('inspect_project', { project: 'moon-island' })) as InspectView;
    expect(view.comparisons[0]!.sides.map(s => s.label)).toEqual(['v6', 'v7']);
    expect(view.stops.stops).toHaveLength(5);
    const character = view.models.find(m => m.model === 'character')!;
    expect(character.preset.lock).toMatch(/medium/);
    await handle('pipeline_save_presets', {
      project: 'moon-island',
      presets: [
        ...((await handle('pipeline_get', { project: 'moon-island' })) as PipelineView).def.presets,
        {
          model: 'flower_shrubs',
          label: 'Shrubs',
          textureSize: 512,
          codec: 'uastc',
          meshopt: 'low',
          lock: null,
        },
      ],
    });
    const after = (await handle('inspect_project', { project: 'moon-island' })) as InspectView;
    const shrubs = after.models.find(m => m.model === 'flower_shrubs')!;
    expect(shrubs.label).toBe('Shrubs');
    expect(shrubs.preset).toMatchObject({ codec: 'uastc', meshopt: 'low', hasRow: true });
  });

  it('runs the pipeline, fails at posters once, then passes from there', async () => {
    const { handle, events } = backend('configured');
    const run = (await handle('pipeline_run', {
      project: 'moon-island',
      from: null,
      force: false,
    })) as RunRecord;
    expect(run.number).toBe(25);
    let view = await finished(handle);
    expect(view.runs[0]!.state).toBe('failed');
    expect(view.runs[0]!.steps.find(s => s.id === 'build')!.state).toBe('cached');
    expect(view.runs[0]!.steps.find(s => s.id === 'ship')!.state).toBe('skipped');
    expect(events.some(e => e.type === 'log' && e.entry.level === 'error')).toBe(true);

    await handle('pipeline_run', { project: 'moon-island', from: 'posters', force: false });
    view = await finished(handle);
    expect(view.runs[0]!.state).toBe('done');
    expect(view.runs[0]!.steps.find(s => s.id === 'export')!.state).toBe('skipped');
  });

  it('passes every step in the idle scenario', async () => {
    const { handle } = backend('idle');
    await handle('pipeline_run', { project: 'moon-island', from: null, force: true });
    const view = await finished(handle);
    expect(view.runs[0]!).toMatchObject({ number: 24, state: 'done' });
    expect(view.runs[0]!.steps.every(s => s.state === 'done')).toBe(true);
  });

  it('refuses edits to a locked preset row', async () => {
    const { handle } = backend();
    const view = (await handle('pipeline_get', { project: 'moon-island' })) as PipelineView;
    const presets = view.def.presets.map(p => (p.lock ? { ...p, meshopt: 'high' } : p));
    await expect(
      handle('pipeline_save_presets', { project: 'moon-island', presets })
    ).rejects.toMatchObject({
      code: 'invalid',
    });
  });

  it('adds and moves assets', async () => {
    const { handle } = backend();
    let project = (await handle('asset_add', {
      project: 'moon-island',
      asset: { name: 'Paper lamp', kind: 'prop', stage: 'concept' },
    })) as ProjectManifest;
    expect(project.assets!.find(a => a.id === 'paper-lamp')).toMatchObject({ stage: 'concept' });
    project = (await handle('asset_move', {
      project: 'moon-island',
      id: 'paper-lamp',
      stage: 'refs',
    })) as ProjectManifest;
    expect(project.assets!.find(a => a.id === 'paper-lamp')!.stage).toBe('refs');
  });

  it('renames an asset and keeps its id', async () => {
    const { handle } = backend();
    await handle('asset_add', {
      project: 'moon-island',
      asset: { name: 'Paper lamp', kind: 'prop', stage: 'concept' },
    });
    await handle('asset_add', {
      project: 'moon-island',
      asset: { name: 'Stone bench', kind: 'prop', stage: 'concept' },
    });
    const project = (await handle('asset_rename', {
      project: 'moon-island',
      id: 'paper-lamp',
      name: ' Hanging lamp ',
    })) as ProjectManifest;
    expect(project.assets!.find(a => a.id === 'paper-lamp')!.name).toBe('Hanging lamp');
    await expect(
      handle('asset_rename', { project: 'moon-island', id: 'paper-lamp', name: 'stone bench' })
    ).rejects.toMatchObject({ code: 'invalid' });
    await expect(
      handle('asset_rename', { project: 'moon-island', id: 'nope', name: 'x' })
    ).rejects.toMatchObject({ code: 'not-found' });
  });

  it('accepts an inbox model into the project', async () => {
    const { handle, inboxCount } = backend('configured');
    const result = (await handle('inbox_accept', {
      request: { item: 'sakura_large.glb', project: 'moon-island', asset: null },
    })) as AcceptResult;
    expect(result.version.file).toBe('models/sakura_large_v1.glb');
    expect(result.asset).toMatchObject({ stage: 'blender', status: 'review' });
    expect(result.project.assets!.some(a => a.id === result.asset.id)).toBe(true);
    expect(inboxCount()).toBe(1);
    const snapshot = (await handle('inbox_list')) as { items: { id: string }[] };
    expect(snapshot.items.map(i => i.id)).not.toContain('sakura_large.glb');
  });

  it('adds a version to the asset a model was sent for', async () => {
    const { handle } = backend('configured');
    const result = (await handle('inbox_accept', {
      request: { item: 'sakura_large.glb', project: 'moon-island', asset: 'sakura' },
    })) as AcceptResult;
    expect(result.asset.id).toBe('sakura');
    expect(result.version.file).toMatch(/^models\/sakura_v\d+\.glb$/);
  });

  it('saves the real size and fix-ups set before Accept, or refuses them before the move', async () => {
    const { handle } = backend('configured');
    await expect(
      handle('inbox_accept', {
        request: {
          item: 'sakura_large.glb',
          project: 'moon-island',
          asset: null,
          prep: { size: null, fixups: [{ op: 'decimate', triangles: 5 }] },
        },
      })
    ).rejects.toMatchObject({ code: 'invalid' });
    const listed = (await handle('inbox_list')) as { items: { id: string }[] };
    expect(listed.items.map(i => i.id)).toContain('sakura_large.glb');

    const prep = {
      size: { axis: 'height', value: 6, unit: 'm' },
      fixups: [{ op: 'origin-bottom' }, { op: 'merge', byMaterial: false }],
    };
    const result = (await handle('inbox_accept', {
      request: { item: 'sakura_large.glb', project: 'moon-island', asset: null, prep },
    })) as AcceptResult;
    const saved = (await handle('prepare_get', { project: 'moon-island' })) as ScenePrep;
    expect(saved.models[result.asset.id]?.fixups.map(f => f.op)).toEqual([
      'merge',
      'origin-bottom',
    ]);
    const layout = (await handle('layout_get', { project: 'moon-island' })) as LayoutView;
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
      request: { item: 'sakura_large.glb', project: 'moon-island', asset: null },
    })) as AcceptResult;
    expect(result.asset.thumbnail).toBe('thumbs/sakura_large_v1.png');
    expect(aliases).toHaveLength(1);
    expect(aliases[0]![0]).toMatch(/projects\/moon-island\/thumbs\/sakura_large_v1\.png$/);
    expect(aliases[0]![1]).toMatch(/cache\/thumbs\/sakura-large\.webp$/);
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
    const file = snapshot.items.find(i => i.id === 'character.glb')!.file;
    const path = (await handle('thumbnail_store', { file, png: [137, 80, 78, 71] })) as string;
    expect(stored).toEqual([path]);
    const after = (await handle('inbox_list')) as {
      items: { id: string; thumbnail: string | null; thumbnailState: string }[];
    };
    expect(after.items.find(i => i.id === 'character.glb')).toMatchObject({
      thumbnail: path,
      thumbnailState: 'ready',
    });
  });

  it('sends by hand, takes the model back and refuses a send without a key', async () => {
    const { handle } = backend('configured');
    const options = (await handle('provider_send_options', {
      project: 'moon-island',
      asset: 'lanterns',
    })) as SendOptions;
    expect(options.suggested).toEqual(['refs/lanterns_front.png']);
    expect(options.keys.find(k => k.service === 'meshy')?.saved).toBe(false);
    const request = {
      project: 'moon-island',
      asset: 'lanterns',
      provider: 'by-hand',
      service: 'tripo',
      variant: 'prop',
      images: options.suggested,
      tripo: null,
      meshy: null,
    };
    const job = (await handle('provider_send', { request })) as ProviderJob;
    expect(job.state).toBe('waiting');
    expect(job.card?.returnName).toBe(`${job.id}.glb`);
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
    const tripo = (await handle('provider_cancel', { id: 'sakura-1' }).catch(e => e)) as {
      message: string;
    };
    expect(tripo.message).toContain('Tripo cannot cancel');
  });

  it('lists three projects, switches between them and keeps the switch on a settings save', async () => {
    const { handle } = backend('configured');
    const list = (await handle('project_list')) as ProjectSummary[];
    expect(list.map(p => [p.name, p.template])).toEqual([
      ['Moon Island', 'floating-diorama'],
      ['Rooftop cinema', 'room'],
      ['Lantern keeper', 'character-showcase'],
    ]);
    expect(list[0]!.thumb).toBe('thumbs/island.webp');
    const snap = (await handle('project_switch', { id: 'rooftop-cinema' })) as WorkspaceSnapshot;
    expect(snap.project?.id).toBe('rooftop-cinema');
    await handle('workspace_save_config', {
      config: { ...snap.config, currentProject: 'moon-island' },
    });
    const boot = (await handle('app_bootstrap')) as Bootstrap;
    expect(boot.workspace?.project?.id).toBe('rooftop-cinema');
    await expect(handle('project_switch', { id: 'nope' })).rejects.toMatchObject({
      code: 'not-found',
    });
    const pipeline = (await handle('pipeline_get', { project: 'rooftop-cinema' })) as PipelineView;
    expect(pipeline.runs).toEqual([]);
  });

  it('gives the landing studio card its ref and moves the direction to a new project', async () => {
    const { handle } = backend('configured');
    const boot = (await handle('app_bootstrap')) as Bootstrap;
    const card = boot.workspace?.project?.assets?.find(a => a.id === 'maquette-landing-studio');
    expect(card?.thumbnail).toBe('refs/maquette-landing-studio_04.png');
    await handle('project_create', {
      project: {
        name: 'Maquette landing',
        template: 'floating-diorama',
        siteRepo: null,
        sourceFolder: null,
      },
    });
    const moved = (await handle('lab_move_direction', {
      project: 'moon-island',
      direction: 'maquette-landing-studio',
      to: 'maquette-landing',
    })) as MovedDirection;
    expect(moved.direction.images[3]).toMatchObject({ picked: true, starred: true });
    expect(moved.asset?.thumbnail).toBe('refs/maquette-landing-studio_04.png');
    expect((moved.from.assets ?? []).some(a => a.id === 'maquette-landing-studio')).toBe(false);
    expect((moved.to.assets ?? []).map(a => a.id)).toEqual(['maquette-landing-studio']);
    const moon = (await handle('lab_get', { project: 'moon-island' })) as LabView;
    expect(moon.directions.some(d => d.id === 'maquette-landing-studio')).toBe(false);
    const landing = (await handle('lab_get', { project: 'maquette-landing' })) as LabView;
    expect(landing.directions.map(d => d.name)).toEqual(['Maquette landing studio']);
  });
});

describe('mock starter scripts', () => {
  it('updates untouched old starters and replaces an edited one only when asked', async () => {
    const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
    const call = <T>(cmd: string, args: Record<string, unknown> = {}) => handle(cmd, args) as T;
    type View = { starters: { path: string; state: string; backup: string | null }[] };
    const before = await call<View>('pipeline_get', { project: 'moon-island' });
    expect(before.starters.filter(s => s.state === 'outdated')).toHaveLength(3);
    const all = before.starters.map(s => s.path);
    const kept = await call<View>('pipeline_update_starters', {
      project: 'moon-island',
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
      project: 'moon-island',
      scripts: ['tools/export_web.py'],
      replace: true,
    });
    expect(replaced.starters.find(s => s.path === 'tools/export_web.py')).toEqual({
      path: 'tools/export_web.py',
      state: 'current',
      backup: 'tools/export_web.py.bak',
    });
    expect(
      (await call<View>('pipeline_get', { project: 'rooftop-cinema' })).starters.every(
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
    expect(await presetsOf('moon-island')).toContain('character');
    expect(await presetsOf('rooftop-cinema')).toEqual([]);
  });

  it('takes the rows the panel saves back, the locked one included', async () => {
    const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
    const view = (await handle('pipeline_get', { project: 'moon-island' })) as PipelineView;
    const rows = presetsToSave(presetRows(view));
    const saved = (await handle('pipeline_save_presets', {
      project: 'moon-island',
      presets: rows,
    })) as PipelineView;
    expect(saved.def.presets).toEqual(rows);
  });

  it('copies an asset under a free name and counts what the first stop sees', async () => {
    const { handle } = backend('configured');
    const first = (await handle('layout_get', { project: 'moon-island' })) as LayoutView;
    expect(first.firstView.stop).toBe('Porch');
    expect(first.firstView.models.length).toBeGreaterThan(0);
    expect(first.firstView.targetMb).toBe(3);
    const copy = async () =>
      (await handle('asset_copy_to_project', {
        project: 'moon-island',
        id: 'character',
        to: 'rooftop-cinema',
      })) as { asset: { id: string; name: string; versions: { file: string }[] } };
    const one = await copy();
    const two = await copy();
    expect(one.asset.id).toBe('character');
    expect(two.asset).toMatchObject({ id: 'character-2', name: 'Character 2' });
    expect(two.asset.versions[0]!.file).toMatch(/^models\/character_2_v\d+\.glb$/);
    const source = (await handle('layout_get', { project: 'moon-island' })) as LayoutView;
    expect(source.models.some(m => m.asset === 'character')).toBe(true);
    const target = (await handle('layout_get', { project: 'rooftop-cinema' })) as LayoutView;
    expect(target.models.map(m => m.asset)).toEqual(
      expect.arrayContaining(['character', 'character-2'])
    );
  });

  it('keeps material tweaks per project, tidied, and refuses a locked map codec change', async () => {
    const { handle } = backend('configured');
    const none = { tint: null, roughness: null, metalness: null, emissiveStrength: null };
    const stored = await handle('materials_save', {
      project: 'moon-island',
      looks: {
        sakura: { all: { ...none, tint: '#FFC0CB' }, materials: { Leaves: none } },
        hut: { all: none, materials: {} },
      },
    });
    expect(stored).toEqual({
      version: 1,
      models: { sakura: { all: { ...none, tint: '#ffc0cb' }, materials: {} } },
    });
    // Another asset's look lands beside the stored one.
    const both = (await handle('materials_save', {
      project: 'moon-island',
      looks: { hut: { all: { ...none, metalness: 0.5 }, materials: {} } },
    })) as SceneMaterials;
    expect(Object.keys(both.models)).toEqual(['sakura', 'hut']);
    expect(await handle('materials_get', { project: 'rooftop-cinema' })).toEqual({
      version: 1,
      models: {},
    });
    await expect(
      handle('materials_save', {
        project: 'moon-island',
        looks: {
          sakura: { all: { ...none, roughness: { mode: 'multiply', value: 3 } }, materials: {} },
        },
      })
    ).rejects.toMatchObject({ code: 'invalid' });

    const view = (await handle('pipeline_get', { project: 'moon-island' })) as PipelineView;
    const rows = view.def.presets.map(p =>
      p.lock ? { ...p, mapCodecs: { color: null, normal: 'etc1s', other: null } } : p
    );
    await expect(
      handle('pipeline_save_presets', { project: 'moon-island', presets: rows })
    ).rejects.toMatchObject({ code: 'invalid' });
  });
});
