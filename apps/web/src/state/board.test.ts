import { describe, expect, it } from 'vitest';
import { defaultConfig, moonIsland } from '@/ipc/mock/fixtures';
import type { WorkspaceSnapshot } from '@/ipc/client';
import { assetCount, cardNumbers, columns, projectFile } from './board';

const project = moonIsland();

describe('board helpers', () => {
  it('sorts assets into stage columns, newest first', () => {
    const cols = columns(project.assets ?? []);
    expect(Object.keys(cols)).toEqual(['concept', 'refs', 'tripo', 'blender', 'web', 'ship']);
    expect(cols.web.map(a => a.id)).toEqual(['island', 'shop-house']);
    expect(cols.blender.map(a => a.id)).toContain('character');
    expect(cols.ship).toEqual([]);
  });

  it('shows the latest version numbers on a card', () => {
    const character = project.assets!.find(a => a.id === 'character')!;
    expect(cardNumbers(character)).toMatchObject({ version: 2, tris: '94.6k', status: 'review' });
    const concept = project.assets!.find(a => a.stage === 'concept')!;
    expect(cardNumbers(concept).version).toBeUndefined();
  });

  it('counts and resolves files', () => {
    expect(assetCount(0)).toBe('No assets yet');
    expect(assetCount(1)).toBe('1 asset');
    const ws = { root: '/w', config: defaultConfig(), project } as unknown as WorkspaceSnapshot;
    expect(projectFile(ws, 'thumbs/island.jpg')).toBe('/w/projects/moon-island/thumbs/island.jpg');
    const win = { ...ws, root: 'C:\\Maquette' } as WorkspaceSnapshot;
    expect(projectFile(win, 'thumbs/a.jpg')).toBe(
      'C:\\Maquette\\projects\\moon-island\\thumbs\\a.jpg'
    );
  });
});
