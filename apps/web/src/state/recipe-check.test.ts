import { describe, expect, it } from 'vitest';
import type { LayerSettings, RecipeBlank, RecipeLayer } from '@/ipc/client';
import { checkDraft, problemCount } from './recipe-check';
import { newLayer } from './recipe-draft';

const lantern: RecipeBlank = {
  id: 'lantern',
  label: 'Lantern',
  kind: 'prop',
  prop: null,
  point: null,
};

describe('recipe checks', () => {
  it('passes a good draft', () => {
    const draft = { layers: [newLayer('reach', [], [lantern])], blanks: [lantern] };
    expect(problemCount(checkDraft(draft))).toBe(0);
  });

  it('says what the core would refuse, next to the change or blank', () => {
    const reach = newLayer('reach', [], []);
    const look = { ...newLayer('look-at', [reach], [lantern]), bone: 'rightHand' };
    const nudge = newLayer('nudge', [reach, look], []);
    nudge.settings = { ...nudge.settings, rotation: null, offset: null };
    const wave = newLayer('wave', [reach, look, nudge], []);
    wave.settings = { ...wave.settings, amount: 120, start: 0.8, end: 0.4 };
    const blank = { ...lantern, id: 'bad id', label: ' ' };
    const problems = checkDraft({ layers: [reach, look, nudge, wave], blanks: [blank] });
    expect(problems.layers[reach.id]).toBe('A reach needs a target: pick a blank, or add one.');
    expect(problems.layers[look.id]).toMatch(/^A look at moves head, neck/);
    expect(problems.layers[nudge.id]).toBe('A nudge needs a rotation, an offset or both.');
    expect(problems.layers[wave.id]).toBe('It has to start before it ends.');
    expect(problems.blanks['bad id']).toMatch(/is not a blank id/);
  });

  it('catches a target that is not a blank and two blanks with one id', () => {
    const reach = { ...newLayer('reach', [], [lantern]) };
    reach.settings = { ...reach.settings, target: 'gone' };
    const problems = checkDraft({ layers: [reach], blanks: [lantern, lantern] });
    expect(problems.layers[reach.id]).toMatch(/aims at the blank gone/);
    expect(problems.blanks.lantern).toBe('Two blanks are called lantern.');
  });

  it('marks a prop blank holding the model the recipe animates, only when that model is known', () => {
    const self = { ...lantern, prop: 'elsheria' };
    const draft = { layers: [newLayer('reach', [], [self])], blanks: [self] };
    expect(problemCount(checkDraft(draft))).toBe(0);
    const problems = checkDraft(draft, { asset: 'elsheria', name: 'Elsheria' });
    expect(problems.blanks.lantern).toBe(
      'Elsheria is the model this recipe animates. Pick another prop.'
    );
    expect(problemCount(checkDraft(draft, { asset: 'shiro', name: 'Shiro' }))).toBe(0);
  });
});

describe('grip checks', () => {
  const grip = (more: Partial<RecipeLayer> = {}, settings: Partial<LayerSettings> = {}) => {
    const base = newLayer('reach', [], []);
    return {
      ...base,
      id: 'g-1',
      kind: 'grip',
      bone: 'rightHand',
      mode: 'replace',
      pose: { rightHand: [0, 0, 10], rightIndexProximal: [0, 0, 60] },
      ...more,
      settings: { ...base.settings, target: null, easeIn: 4, easeOut: 4, ...settings },
    } as RecipeLayer;
  };
  const problem = (layer: RecipeLayer, blanks: RecipeBlank[] = []) =>
    checkDraft({ layers: [layer], blanks }).layers['g-1'] ?? null;

  it('passes a grip with a pose, whole frame eases and no blank', () => {
    expect(problem(grip())).toBeNull();
  });

  it('says what the core would refuse', () => {
    expect(problem(grip({}, { target: 'staff' }))).toMatch(/holds the blank staff/);
    expect(problem(grip({ pose: null }))).toMatch(/needs its pose/);
    expect(problem(grip({ pose: {} }))).toMatch(/turns no bone/);
    expect(problem(grip({ pose: { rightHand: [0, 200, 0] } }))).toMatch(/more than 180 degrees/);
    expect(problem(grip({ pose: { rightTail: [0, 0, 0] } }))).toMatch(/not a humanoid bone/);
    expect(problem(grip({}, { easeIn: 2.5 }))).toMatch(/whole frames/);
    expect(problem(grip({}, { easeOut: 100_001 }))).toMatch(/0 to 100000/);
    expect(problem(grip({}, { easeOut: 100_000 }))).toBeNull();
    expect(problem(grip({ bone: 'chest' }))).toMatch(/moves leftHand, rightHand/);
  });
});

describe('IK overlap checks', () => {
  const ik = (id: string, kind: 'pin' | 'reach', bone: string, start: number, end: number) => {
    const base = newLayer('reach', [], [lantern]);
    return {
      ...base,
      id,
      kind,
      bone,
      settings: { ...base.settings, start, end, target: kind === 'reach' ? 'lantern' : null },
    } as RecipeLayer;
  };
  const problems = (layers: RecipeLayer[]) => checkDraft({ layers, blanks: [lantern] }).layers;

  it('refuses a pin holding a bone over shares another pin or a reach on it holds, as the core does', () => {
    const found = problems([
      ik('l-1', 'pin', 'leftFoot', 0.1, 0.5),
      ik('l-2', 'pin', 'leftFoot', 0.4, 0.8),
    ]);
    expect(found['l-1']).toBeUndefined();
    expect(found['l-2']).toBe(
      'The pin l-1 and the pin l-2 both hold leftFoot with IK over the same part of the recipe. Move one so they do not overlap, or take one out.'
    );
    const reach = problems([
      ik('l-1', 'reach', 'rightHand', 0.3, 0.6),
      ik('l-2', 'pin', 'rightHand', 0.5, 0.9),
    ]);
    expect(reach['l-2']).toMatch(/^The reach l-1 and the pin l-2/);
  });

  it('lets IK changes apart in time, on two bones, or two reaches through', () => {
    const found = problems([
      ik('l-1', 'pin', 'leftFoot', 0.1, 0.4),
      ik('l-2', 'pin', 'leftFoot', 0.4, 0.8),
      ik('l-3', 'pin', 'rightFoot', 0.1, 0.8),
      ik('l-4', 'reach', 'rightHand', 0.2, 0.6),
      ik('l-5', 'reach', 'rightHand', 0.3, 0.7),
    ]);
    expect(found).toEqual({});
  });
});
