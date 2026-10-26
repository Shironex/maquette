import { describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMotion } from '@/test/open-motion';
import { cabinetStates, motionFiles } from './create-motion-handlers';
import { afterEach } from 'vitest';

afterEach(() => clearMocks());

describe('motion in mock mode', () => {
  it('serves the demo models, the rest figure only for rest', () => {
    expect(motionFiles(null)).toEqual({});
    expect(Object.values(motionFiles('still'))).toEqual(['switch_cabinet.glb']);
    expect(motionFiles('rest')['models/clay_figure_v2.glb']).toBe('clay_figure_rest.glb');
  });

  it('weighs the clips inside the scene and saves states and clip choices', async () => {
    const s = await openMotion();
    const figure = s.view!.models.find(m => m.asset === 'clay-figure')!;
    expect(figure.clips.map(c => c.name)).toEqual(['Idle breathe', 'Walk loop', 'Wave']);
    expect(s.view!.budget.motionBytes).toBeGreaterThan(0);
    expect(s.view!.states['switch-cabinet']).toEqual(cabinetStates());
    const saved = await commands.motionSave('moon-island', {}, { 'clay-figure': ['Wave'] });
    expect(saved.states).toEqual({});
    expect(saved.clipsOff).toEqual({ 'clay-figure': ['Wave'] });
    expect(saved.clipsOffSupported).toBe(true);
  });

  it('tightens the scene budget for over', async () => {
    const s = await openMotion('over');
    expect(s.view!.budget.sceneTargetMb).toBe(2.25);
  });
});
