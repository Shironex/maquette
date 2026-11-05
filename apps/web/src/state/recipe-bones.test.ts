import { describe, expect, it } from 'vitest';
import { HUMAN_BONES, boneWords, bonesFor, limbOf, mixamoName } from './recipe-bones';

describe('recipe bones', () => {
  it('lists the 55 VRM humanoid bones, and each kind keeps to its own', () => {
    expect(HUMAN_BONES).toHaveLength(55);
    expect(bonesFor('reach')).toEqual(['leftHand', 'rightHand', 'leftFoot', 'rightFoot']);
    expect(bonesFor('look-at')).toEqual(['head', 'neck']);
    expect(bonesFor('wave')).toContain('rightLowerArm');
    expect(bonesFor('nudge')).toBe(HUMAN_BONES);
  });

  it('names each bone as the bone map lists it', () => {
    expect(mixamoName('rightLowerArm')).toBe('RightForeArm');
    expect(mixamoName('chest')).toBe('Spine1');
    expect(mixamoName('leftThumbMetacarpal')).toBe('LeftHandThumb1');
    expect(mixamoName('rightLittleDistal')).toBe('RightHandPinky3');
    expect(mixamoName('jaw')).toBeNull();
  });

  it('finds the limb a reach bends and reads bone names in words', () => {
    expect(limbOf('leftFoot')).toEqual({ upper: 'leftUpperLeg', lower: 'leftLowerLeg' });
    expect(limbOf('head')).toBeNull();
    expect(boneWords('leftUpperArm')).toBe('Left upper arm');
  });
});
