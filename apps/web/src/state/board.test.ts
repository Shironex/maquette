import { nothingToRun } from './board';
import type { ProjectManifest, StepDef } from '@/ipc/client';
import { describe, expect, it } from 'vitest';
import { defaultConfig, frostveilProject } from '@/ipc/mock/fixtures';
import type { WorkspaceSnapshot } from '@/ipc/client';
import { assetCount, cardNumbers, columns, projectFile } from './board';

const project = frostveilProject();

describe('board helpers', () => {
  it('sorts assets into stage columns, newest first', () => {
    const cols = columns(project.assets ?? []);
    expect(Object.keys(cols)).toEqual(['concept', 'refs', 'tripo', 'blender', 'web', 'ship']);
    expect(cols.web.map(a => a.id)).toEqual(['observatory-arch']);
    expect(cols.blender.map(a => a.id)).toContain('elsheria');
    expect(cols.ship.map(a => a.id)).toEqual(['veil-platform']);
    expect(cols.concept.map(a => a.id)).toEqual(['rune-obelisk', 'frozen-angel']);
  });

  it('shows the latest version numbers on a card', () => {
    const character = project.assets!.find(a => a.id === 'elsheria')!;
    expect(cardNumbers(character)).toMatchObject({ version: 2, tris: '115.7k', status: 'review' });
    const concept = project.assets!.find(a => a.stage === 'concept')!;
    expect(cardNumbers(concept).version).toBeUndefined();
  });

  it('counts and resolves files', () => {
    expect(assetCount(0)).toBe('No assets yet');
    expect(assetCount(1)).toBe('1 asset');
    const ws = { root: '/w', config: defaultConfig(), project } as unknown as WorkspaceSnapshot;
    expect(projectFile(ws, 'thumbs/island.jpg')).toBe('/w/projects/frostveil/thumbs/island.jpg');
    const win = { ...ws, root: 'C:\\Maquette' } as WorkspaceSnapshot;
    expect(projectFile(win, 'thumbs/a.jpg')).toBe(
      'C:\\Maquette\\projects\\frostveil\\thumbs\\a.jpg'
    );
  });
});

describe('nothing to run', () => {
  const noModels = { id: 'p', template: 'room', assets: [] } as unknown as ProjectManifest;
  const step = (id: string, kind: StepDef['kind'], script: string | null) =>
    ({ id, kind, script }) as unknown as StepDef;
  const pipeline = (steps: StepDef[], starters: string[]) => ({
    def: { steps },
    starters: starters.map(path => ({ path, state: 'current', backup: null })),
  });

  it("holds a run with no models when every step is the template's own", () => {
    const own = pipeline(
      [step('a', 'blender', 'tools/build.py'), step('b', 'gltf-transform', null)],
      ['tools/build.py']
    );
    expect(nothingToRun(noModels, own as never)).toMatch(/No models/);
    expect(nothingToRun(noModels)).toMatch(/No models/);
  });

  it('lets a project with a script step of its own run', () => {
    const custom = pipeline([step('a', 'blender', 'tools/mine.py')], ['tools/build.py']);
    expect(nothingToRun(noModels, custom as never)).toBeNull();
  });

  it('lets a project with a shell step of its own run', () => {
    const custom = pipeline([step('a', 'shell', null)], []);
    expect(nothingToRun(noModels, custom as never)).toBeNull();
  });

  it('exempts an imported folder, which builds from its own scene', () => {
    expect(nothingToRun({ ...noModels, template: 'imported' })).toBeNull();
  });
});
