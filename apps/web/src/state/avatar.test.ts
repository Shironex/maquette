import { describe, expect, it } from 'vitest';
import { vrmReadiness } from '@/ipc/mock/vrm-fixtures';
import {
  boneGroups,
  boneLabel,
  buildAction,
  measureRows,
  metaProblem,
  parseAuthors,
  poseSummary,
  specFromDefaults,
} from './avatar';

describe('the avatar helpers', () => {
  it('start a spec from the workspace defaults, strictest licence when unset', () => {
    const spec = specFromDefaults({ author: '  Ana ', commercialUsage: 'corporation' }, ' Hero ');
    expect(spec.meta).toMatchObject({
      name: 'Hero',
      authors: ['Ana'],
      avatarPermission: 'onlyAuthor',
      commercialUsage: 'corporation',
      licenseUrl: 'https://vrm.dev/licenses/1.0/',
    });
    expect(spec.material).toBe('mtoon-soft');
    expect(specFromDefaults(undefined, 'Hero').meta.authors).toEqual([]);
  });

  it('say what a build would refuse in the meta', () => {
    const meta = specFromDefaults({ author: 'Ana' }, 'Hero').meta;
    expect(metaProblem(meta)).toBeNull();
    expect(metaProblem({ ...meta, name: ' ' })).toBe('The avatar needs a name.');
    expect(metaProblem({ ...meta, authors: [] })).toBe('Name at least one author.');
    expect(parseAuthors('Ana, , Studio North,')).toEqual(['Ana', 'Studio North']);
  });

  it('group the bones and name them for reading', () => {
    const r = vrmReadiness({ hips: 'Hips', leftThumbProximal: 'Thumb_L' }, 'vrm', []);
    const [required, body, fingers] = boneGroups(r);
    expect([required!.bones.length, body!.bones.length, fingers!.bones.length]).toEqual([
      15, 10, 30,
    ]);
    expect([required!.mapped, fingers!.mapped]).toEqual([1, 1]);
    expect(boneLabel('leftUpperArm')).toBe('Left upper arm');
  });

  it('measure the pose before and after, marking what reached VRM', () => {
    const rows = measureRows(
      { facingDeg: 90, armLeftDeg: 55, groundY: 0.03, height: 1.62 },
      { facingDeg: 0, armLeftDeg: 0.4, groundY: 0, height: 1.62 }
    );
    expect(rows.find(r => r.id === 'facing')).toMatchObject({
      before: '90°',
      after: '0°',
      fixed: true,
    });
    expect(rows.find(r => r.id === 'ground')).toMatchObject({ before: '3 cm', after: '0 cm' });
    expect(measureRows({ facingDeg: 90 })).toEqual([
      expect.objectContaining({ id: 'facing', after: null }),
    ]);
    expect(
      poseSummary({
        turnedDeg: 90,
        elbows: 'straighten',
        lifted: { left: { shoulderDeg: 55 }, right: { shoulderDeg: 53 } },
      })
    ).toBe('Turned 90°, lifted the left arm 55° and the right arm 53°, elbows straightened.');
  });

  it('hold the build until the meta and the pose allow it', () => {
    const readiness = vrmReadiness({}, 'vrm', []);
    const spec = specFromDefaults({ author: 'Ana' }, 'Hero');
    const view = {
      project: 'p',
      asset: 'hero',
      name: 'Hero',
      kind: 'character' as const,
      version: 1,
      model: '/m.glb',
      spec,
      stored: true,
      readiness,
      readinessError: null,
      needsNormalize: true,
      normalized: null,
      export: null,
      exportPath: '/p/exports/hero.vrm',
      metaProblem: null,
    };
    expect(buildAction(view, spec)).toMatchObject({ label: 'Build VRM', disabled: false });
    expect(buildAction(view, { ...spec, meta: { ...spec.meta, authors: [] } })).toMatchObject({
      disabled: true,
      tooltip: 'Name at least one author',
    });
    expect(buildAction({ ...view, readiness: null, readinessError: 'No rig' }, spec).tooltip).toBe(
      'No rig'
    );
  });
});
