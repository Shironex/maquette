import { describe, expect, it } from 'vitest';
import type { AcceptResult, PipelineView, ScenePrep } from '../client';
import { createBackend } from './backend';

function backend() {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
  const call = <T>(cmd: string, args: Record<string, unknown> = {}) =>
    handle(cmd, args) as Promise<T>;
  const bake = async (project = 'moon-island') =>
    (await call<PipelineView>('pipeline_get', { project })).bake!;
  return { call, bake };
}

describe('mock bake settings', () => {
  it('reads an older bake script as it bakes and refuses denoise until it is updated', async () => {
    const { call, bake } = backend();
    const before = await bake();
    expect(before.script).not.toBe('current');
    expect(before.canDenoise).toBe(false);
    expect(before.settings).toEqual({ size: 512, samples: 16, denoise: false });
    await expect(
      call('pipeline_save_bake', {
        project: 'moon-island',
        patch: { size: null, samples: null, denoise: true },
      })
    ).rejects.toMatchObject({ code: 'invalid' });

    await call('pipeline_update_starters', {
      project: 'moon-island',
      scripts: ['tools/bake_light.py'],
      replace: false,
    });
    const after = await bake();
    expect(after.canDenoise).toBe(true);
    expect(after.settings).toEqual({ size: 1024, samples: 32, denoise: true });
  });

  it('changes only the given fields and refuses sizes and samples the core refuses', async () => {
    const { call, bake } = backend();
    const project = 'rooftop-cinema';
    const saved = await call<PipelineView>('pipeline_save_bake', {
      project,
      patch: { size: 2048, samples: null, denoise: null },
    });
    expect(saved.bake!.settings).toEqual({ size: 2048, samples: 32, denoise: true });
    for (const patch of [
      { size: 1000, samples: null, denoise: null },
      { size: null, samples: 0, denoise: null },
    ])
      await expect(call('pipeline_save_bake', { project, patch })).rejects.toMatchObject({
        code: 'invalid',
      });
    expect((await bake(project)).settings.size).toBe(2048);
  });

  it('skips the bake for an accepted Inbox model unless someone chose', async () => {
    const { call, bake } = backend();
    const accept = (item: string, prep: unknown) =>
      call<AcceptResult>('inbox_accept', {
        request: { item, project: 'moon-island', asset: null, prep },
      });
    const scan = await accept('sakura_large.glb', null);
    const prep = await call<ScenePrep>('prepare_get', { project: 'moon-island' });
    expect(prep.models[scan.asset.id]?.skipBake).toBe(true);
    expect(prep.models[scan.asset.id]?.padEdges).toBe(true);
    expect((await bake()).skipped).toContain(scan.asset.id);
  });
});
