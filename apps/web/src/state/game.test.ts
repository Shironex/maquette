import { describe, expect, it } from 'vitest';
import type { Asset, GameSettings, ShipFile, ShipPlan } from '@/ipc/client';
import {
  budgetFor,
  budgetSummary,
  checkAsset,
  fileOf,
  fixName,
  folderProblem,
  gameOf,
  shipCounts,
  shipGroups,
  stageLabels,
  trianglesLine,
} from './game';
import { gameProject } from '@/ipc/mock/game-fixtures';

const settings: GameSettings = {
  format: 'glb',
  modelsDir: 'assets/models',
  export: { textures: 'png', textureCap: 1024 },
  bake: false,
  budgets: [
    { folder: 'props', triangles: 12_000, texture: 1024 },
    { folder: 'characters', triangles: 60_000, texture: 2048 },
  ],
  models: { 'iron-gate': { file: 'props/doors/iron_gate.glb', importedVersion: 1 } },
};

function asset(id: string, triangles: number, maxTexture = 1024, kind = 'prop'): Asset {
  return {
    id,
    name: id,
    kind: kind as Asset['kind'],
    stage: 'web',
    status: 'done',
    versions: [
      {
        version: 1,
        file: `models/${id}_v1.glb`,
        createdAt: '',
        stats: { triangles, bytes: 1e6, textures: 1, maxTexture, joints: 0, animations: 0 },
      },
    ],
    thumbnail: null,
    createdAt: '',
    updatedAt: '',
  };
}

describe('game projects', () => {
  it('calls the web stage Export for a game', () => {
    expect(stageLabels('game').web).toBe('Export');
    expect(stageLabels('website').web).toBe('Web');
    expect(stageLabels(undefined).ship).toBe('Ship');
  });

  it('places a model by its record, else by its kind', () => {
    expect(fileOf(settings, asset('iron-gate', 1))).toBe('props/doors/iron_gate.glb');
    expect(fileOf(settings, asset('candle-cluster', 1))).toBe('props/candle_cluster.glb');
    expect(fileOf(settings, asset('heroine', 1, 1024, 'character'))).toBe('characters/heroine.glb');
    const moved = { ...settings, folders: { 'iron-gate': 'kit', 'candle-cluster': 'kit/lights' } };
    expect(fileOf(moved, asset('iron-gate', 1))).toBe('kit/iron_gate.glb');
    expect(fileOf(moved, asset('candle-cluster', 1))).toBe('kit/lights/candle_cluster.glb');
  });

  it('takes a game folder only inside the model folder', () => {
    expect(folderProblem('kit/')).toBeNull();
    expect(folderProblem('')).toBeNull();
    expect(folderProblem('/etc')).not.toBeNull();
    expect(folderProblem('C:/games')).not.toBeNull();
    expect(folderProblem('kit/../up')).toBe('No .. or empty parts');
    expect(fileOf({ ...settings, budgets: [] }, asset('crate', 1))).toBe('crate.glb');
  });

  it('takes the budget of the nearest folder up', () => {
    expect(budgetFor(settings, 'props/doors/iron_gate.glb')?.folder).toBe('props');
    expect(budgetFor(settings, 'loose.glb')).toBeNull();
  });

  it('checks triangles and texture size against the budget', () => {
    const over = checkAsset(settings, asset('iron-gate', 14_600, 2048))!;
    expect(over.problems).toEqual(['Over 12k triangles', 'Texture over 1024 px']);
    expect(trianglesLine(over)).toBe('14.6k / 12k');
    expect(fixName(over)).toBe('Decimate');
    expect(fixName({ ...over, triangles: 100 })).toBe('Shrink');
    expect(checkAsset(settings, asset('chest', 5_000))?.problems).toEqual([]);
  });

  it('counts the models over budget in a game and nothing for a website', () => {
    const crypt = gameProject('crypt', true);
    const summary = budgetSummary(crypt)!;
    expect(summary.models).toBe(15);
    expect(summary.over.map(a => a.id)).toEqual(['banner', 'iron-gate', 'knight']);
    expect(summary.overTriangles + summary.overTextures).toBeGreaterThanOrEqual(3);
    expect(gameOf({ ...crypt, target: 'website' })).toBeNull();
    expect(budgetSummary({ ...crypt, target: 'website' })).toBeNull();
  });

  it('groups the Ship plan and never removes a file', () => {
    const file = (path: string, change: ShipFile['change'], blocked = false): ShipFile => ({
      path,
      change,
      bytes: 1,
      beforeBytes: null,
      triangles: null,
      beforeTriangles: null,
      asset: null,
      blocked,
      imported: false,
    });
    const plan: ShipPlan = {
      repo: '/game',
      modelsDir: 'assets/models',
      branch: 'main',
      exported: true,
      files: [
        file('a.glb', 'new'),
        file('b.glb', 'replaced'),
        file('c.glb', 'unchanged'),
        file('d.glb', 'new', true),
        file('props/old.glb', 'removed'),
      ],
      handMade: [file('e.glb', 'hand-made')],
      recordAfter: 3,
      checks: [],
      uncommitted: null,
    };
    const groups = shipGroups(plan);
    expect(groups.added.map(f => f.path)).toEqual(['a.glb']);
    expect(groups.blocked.map(f => f.path)).toEqual(['d.glb']);
    expect(groups.handMade).toHaveLength(1);
    expect(groups.removed.map(f => f.path)).toEqual(['props/old.glb']);
    expect(shipCounts(groups)).toBe('1 new · 1 replaced · 1 removed');
  });
});
