import { describe, expect, it } from 'vitest';
import type { Asset, AssetLink, Direction, DirectionLink, PlanNode } from '@/ipc/client';
import { anyRunning, assetView, directionView, linkedIds, pickRows } from './plan-links';
import { newNode } from './plans';

const asset = (state: AssetLink['state'], more: Partial<AssetLink> = {}): AssetLink => ({
  id: 'warden',
  state,
  name: state === 'gone' ? null : 'Frost Warden',
  kind: state === 'gone' ? null : 'character',
  stage: state === 'gone' ? null : 'blender',
  thumbnail: null,
  version: null,
  file: null,
  stats: null,
  checks: 0,
  ...more,
});

const direction = (
  state: DirectionLink['state'],
  more: Partial<DirectionLink> = {}
): DirectionLink => ({
  id: 'lake',
  state,
  name: state === 'gone' ? null : 'Frozen lake',
  images: 6,
  picked: 0,
  pending: 0,
  done: 0,
  thumbnail: null,
  ...more,
});

describe('linkedIds', () => {
  const at = { x: 0, y: 0 };
  it('collects each linked asset and direction once, sorted', () => {
    const nodes: PlanNode[] = [
      newNode('asset', at, { asset: 'warden' }, 1),
      newNode('asset', at, { asset: 'arch' }, 1),
      newNode('asset', at, { asset: 'warden' }, 1),
      newNode('direction', at, { direction: 'lake' }, 1),
      newNode('note', at, {}, 1),
    ];
    expect(linkedIds(nodes)).toEqual({ assets: ['arch', 'warden'], directions: ['lake'] });
  });
});

describe('assetView', () => {
  it('offers each state its next step', () => {
    const ready = assetView(
      asset('ready', {
        version: 3,
        stats: {
          triangles: 18_400,
          bytes: 2_600_000,
          textures: 1,
          maxTexture: 1024,
          joints: 20,
          animations: 0,
        },
      })
    );
    expect(ready).toMatchObject({ status: 'Model ready, v3', tone: 'ok', fact: 'character' });
    expect(ready.detail).toMatch(/^18\.4k tris · .+ · rigged$/);
    expect(ready.action.to).toBe('inspector');

    expect(assetView(asset('check', { checks: 2 }))).toMatchObject({
      status: 'Model to check',
      detail: '2 models in the Inbox',
      action: { to: 'inbox' },
    });
    expect(assetView(asset('no-model')).action.to).toBe('lab');
    expect(assetView(asset('file-missing', { file: 'models/warden_v2.glb' }))).toMatchObject({
      tone: 'bad',
      detail: 'models/warden_v2.glb',
      action: { to: 'board' },
    });
  });

  it('shows a removed asset as gone with its id, and waits for the first read', () => {
    expect(assetView(asset('gone'))).toMatchObject({
      name: 'Asset not found',
      status: 'Not on the Board',
      detail: 'warden',
      gone: true,
      help: expect.stringContaining('Restore it from Removed'),
    });
    expect(assetView(undefined)).toMatchObject({ status: 'Reading...', gone: false });
  });
});

describe('directionView', () => {
  it('reads a round, picks and an empty or gone direction', () => {
    expect(directionView(direction('running', { pending: 2, done: 1 }))).toMatchObject({
      status: 'Lab round running',
      tone: 'run',
      detail: '1 of 3 images',
    });
    expect(directionView(direction('picked', { picked: 1 })).detail).toBe('1 of 6 images picked');
    expect(directionView(direction('pick')).status).toBe('Pick images');
    expect(directionView(direction('empty', { images: 0 })).detail).toBeNull();
    expect(directionView(direction('gone'))).toMatchObject({
      fact: '',
      gone: true,
      help: expect.stringContaining('removed from the Lab'),
    });
    expect(directionView(direction('pick')).action.to).toBe('lab');
  });

  it('says when a linked round runs', () => {
    expect(anyRunning({ lake: direction('running', { pending: 1 }) })).toBe(true);
    expect(anyRunning({ lake: direction('picked') })).toBe(false);
  });
});

describe('pickRows', () => {
  const source = {
    assets: [
      { id: 'warden', name: 'Frost Warden', kind: 'character', stage: 'blender', thumbnail: null },
    ] as unknown as Asset[],
    directions: [
      {
        id: 'lake',
        name: 'Frozen lake',
        images: [
          { file: 'concepts/lake/lake_01.png', index: 1, picked: false },
          { file: 'concepts/lake/lake_02.png', index: 2, picked: true },
          { file: 'concepts/lake/lake_03.png', index: 3, picked: false },
        ],
      },
    ] as unknown as Direction[],
    stageLabels: { blender: 'Blender' } as never,
  };

  it('lists both, marks the linked ones and searches by name', () => {
    const rows = pickRows(source, '', { assets: ['warden'], directions: [] });
    expect(rows.map(r => [r.kind, r.id, r.linked])).toEqual([
      ['asset', 'warden', true],
      ['direction', 'lake', false],
    ]);
    expect(rows[0]?.meta).toBe('character · Blender');
    expect(rows[1]).toMatchObject({ meta: '3 images', picture: 'concepts/lake/lake_02.png' });
    expect(pickRows(source, '  FROZEN ', { assets: [], directions: [] }).map(r => r.id)).toEqual([
      'lake',
    ]);
  });
});
