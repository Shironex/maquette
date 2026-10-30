import { describe, expect, it } from 'vitest';
import type { AddManyPlan } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import { defaultChoice, plannedTags, renamedName, summarise } from './add-many';

async function plan(): Promise<AddManyPlan> {
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    library: 'scale',
  });
  return (await handle('library_add_many_plan', { project: 'hollow-vale' })) as AddManyPlan;
}

describe('adding a whole project', () => {
  it('sorts the models into new, clashes, same file and unreadable', async () => {
    const p = await plan();
    const sum = summarise(p);
    expect(p.assets).toHaveLength(121);
    expect(p.tag).toBe('hollow-vale');
    expect(sum.sameFile).toHaveLength(6);
    expect(sum.unreadable.map(a => a.asset)).toEqual(['barrel-stack', 'fence-corner']);
    expect(sum.clashes.map(a => [a.asset, a.entry])).toEqual([
      ['street-lamp', 'street-lamp'],
      ['stone-well', 'stone-well'],
      ['paper-lanterns', 'paper-lanterns'],
      ['farm-table', 'farm-table'],
    ]);
    expect(sum.adding).toHaveLength(109);
    expect(sum.kinds).toEqual([
      ['prop', 106],
      ['character', 6],
      ['scene', 1],
    ]);
  });

  it('offers a version when it looks the same and keep both when it does not', async () => {
    const p = await plan();
    const choice = (id: string) => defaultChoice(p.assets.find(a => a.asset === id)!);
    expect(choice('street-lamp')).toBe('version');
    expect(choice('stone-well')).toBe('rename');
    expect(choice('mill-door')).toBeNull();
  });

  it('tags each model with the project tag and its kind, and numbers a kept name', () => {
    const options = {
      projectTag: true,
      tag: 'Hollow-Vale',
      collection: '',
      newCollection: '',
      kindTags: { prop: 'prop, Village', character: 'character', scene: '' },
    };
    expect(plannedTags('prop', options)).toEqual(['hollow-vale', 'prop', 'village']);
    expect(plannedTags('scene', { ...options, projectTag: false })).toEqual([]);
    expect(renamedName('Stone well', ['Stone well', 'stone well 2'])).toBe('Stone well 3');
  });
});
