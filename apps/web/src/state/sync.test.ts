import { describe, expect, it } from 'vitest';
import type { ProjectManifest } from '@/ipc/client';
import { conflictKey, conflictTitle, needsRepo, repoQuestion, stignore } from './sync';

const copy = (original: string) => ({
  path: original.replace(/\.json$/, '.sync-conflict-20261027-101500-PCSTUDIO.json'),
  original,
});

const project = (patch: Partial<ProjectManifest>) =>
  ({
    id: 'studio',
    name: 'Studio',
    target: 'website',
    siteRepo: null,
    siteRepoName: 'studio-site',
    siteRepoState: 'elsewhere',
    ...patch,
  }) as ProjectManifest;

describe('sharing a workspace', () => {
  it('names one conflict copy or counts several', () => {
    expect(conflictTitle([copy('projects/studio/project.json')])).toBe(
      'Syncthing kept a conflict copy of project.json'
    );
    const two = [copy('maquette.json'), copy('library/lamp/entry.json')];
    expect(conflictTitle(two)).toBe('Syncthing kept 2 conflict copies of Maquette files');
    expect(conflictKey(two)).not.toBe(conflictKey(two.slice(1)));
  });

  it('recommends an .stignore that keeps the lock synced', () => {
    const text = stignore('cache');
    expect(text).toContain('/cache\n');
    expect(text).toContain('(?d).DS_Store');
    expect(text).not.toMatch(/^[^/].*maquette-lock/m);
  });

  it('asks for a repo picked elsewhere or gone, never for none', () => {
    expect(needsRepo(project({}))).toBe(true);
    expect(needsRepo(project({ siteRepoState: 'gone', siteRepo: '/old/site' }))).toBe(true);
    expect(needsRepo(project({ siteRepoState: 'here', siteRepo: '/x' }))).toBe(false);
    expect(needsRepo(project({ siteRepoState: 'none' }))).toBe(false);
    expect(needsRepo(null)).toBe(false);
  });

  it('words the question for a site or a game', () => {
    expect(repoQuestion(project({}))).toBe(
      "Studio's site repo (studio-site) was picked on another computer. Choose where it is on this one so Ship, the Page view and Preview on site can find it. Each computer keeps its own path."
    );
    expect(
      repoQuestion(project({ target: 'game', siteRepoState: 'gone', siteRepo: 'D:\\game' }))
    ).toBe(
      "Studio's game repo (studio-site) is not at D:\\game any more. Choose where it is now so Import models, the Game export and Ship can find it."
    );
  });
});
