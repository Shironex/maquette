import { describe, expect, it } from 'vitest';
import { gripBoneGroups } from './grip-bones';
import { gripBones } from './recipe-grips';

describe('gripBoneGroups', () => {
  it('groups both hands, the holding side first', () => {
    const bones = [...gripBones('left'), ...gripBones('right')];
    const groups = gripBoneGroups(bones);
    expect(groups.map(g => `${g.label} ${g.bones.length}`)).toEqual([
      'Right hand 1',
      'Right fingers 15',
      'Right forearm 1',
      'Right upper arm 1',
      'Left hand 1',
      'Left fingers 15',
      'Left forearm 1',
      'Left upper arm 1',
    ]);
    expect(gripBoneGroups(bones, 'left')[0]?.label).toBe('Left hand');
  });

  it('leaves out empty groups and bones on neither side', () => {
    const groups = gripBoneGroups(['rightHand', 'rightIndexProximal', 'chest']);
    expect(groups.map(g => g.label)).toEqual(['Right hand', 'Right fingers']);
  });
});
