import { describe, expect, it } from 'vitest';
import type { GameRepoInspection, RepoInspection } from '@/ipc/client';
import { inspectGameRepo } from '@/ipc/mock/game-fixtures';
import { LEGACY_SITE, gameRepoLines, listWords, siteRepoLines } from './repo-lines';

const site: RepoInspection = {
  path: '/site',
  exists: true,
  isGit: true,
  hasPublicDir: false,
  site: { config: true, models: 'static/3d', posters: null, data: 'src/data', error: null },
  looksLikeGame: false,
};

describe('the repo check lines', () => {
  it('names the site folders from the repo, never a fixed island path', () => {
    const lines = siteRepoLines(site).map(l => l.text);
    expect(lines).toContain(
      'Ship writes models to static/3d/, no posters, scene data to src/data/.'
    );
    expect(lines.join(' ')).not.toContain('public/island');
    const legacy = siteRepoLines({ ...site, site: LEGACY_SITE }).map(l => l.text);
    expect(legacy).toContain('No maquette.site.json at the repo root.');
    expect(legacy).toContain(
      'Ship writes models to public/models/ and posters to public/posters/, and no scene data.'
    );
  });

  it('suggests Game for a site repo with a CMakeLists.txt', () => {
    const lines = siteRepoLines({ ...site, looksLikeGame: true });
    expect(lines.at(-1)?.detail).toBe('If it is a game, pick Game under Ships to.');
  });

  it('says what it found in a game repo', () => {
    const repo = inspectGameRepo('/projects/crypt-game');
    const lines = gameRepoLines(repo, 'glb', 'assets/models');
    expect(lines.map(l => l.text)).toEqual([
      'Git repo found.',
      'CMake project found.',
      'Models are .glb in assets/models/',
      'The engine cannot read KTX2 and meshopt.',
      'Ship writes to assets/models/ and only replaces files it wrote.',
    ]);
    expect(lines[1]?.detail).toBe('CMakeLists.txt builds crypt and crypt_tests.');
    expect(lines[2]?.detail).toBe(
      '(16 files in props/, characters/ and enemies/), read with cgltf.'
    );
    expect(lines[3]?.detail).toBe('Game export writes plain .glb with PNG and JPG textures.');
  });

  it('warns when the repo has no models or no CMake', () => {
    const empty: GameRepoInspection = {
      ...inspectGameRepo('/projects/crypt-game'),
      cmake: false,
      modelsDir: null,
      format: null,
      glbFiles: 0,
    };
    const lines = gameRepoLines(empty, 'obj', 'assets/models');
    expect(lines.map(l => l.text)).toContain('No CMakeLists.txt at the root.');
    expect(lines.find(l => l.text === 'No models found.')?.detail).toBe(
      'Ship writes to assets/models/.'
    );
    expect(gameRepoLines({ ...empty, exists: false }, 'glb', 'x')).toEqual([
      { tone: 'missing', text: 'Folder not found.' },
    ]);
  });

  it('joins words', () => {
    expect(listWords(['a'])).toBe('a');
    expect(listWords(['a', 'b', 'c'])).toBe('a, b and c');
  });
});
