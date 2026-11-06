import { describe, expect, it } from 'vitest';
import type { RecipeBlank } from '@/ipc/client';
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
