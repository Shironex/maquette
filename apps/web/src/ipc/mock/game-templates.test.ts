import { describe, expect, it } from 'vitest';
import type { GameSettings } from '../client';
import { gameProject } from './game-fixtures';
import { templateOutcome, templatePreview } from './game-templates';

describe('mock game templates', () => {
  const project = gameProject('crypt', true);
  const settings = project.game as GameSettings;

  it('keeps other rows and lists only the new one', () => {
    const preview = templatePreview(project, settings, 'level-kit', new Set());
    expect(preview.budgets.map(b => b.folder)).toEqual(['kit']);
    expect(preview.changes).toBe(true);
    const { next } = templateOutcome(project, settings, 'level-kit', new Set());
    expect(next.budgets.map(b => b.folder)).toEqual(['props', 'characters', 'enemies', 'kit']);
  });

  it('finds nothing to do when the game has the template already', () => {
    expect(templatePreview(project, settings, 'prop-set', new Set()).changes).toBe(false);
  });

  it('warns when the game holds another format', () => {
    const notes = templatePreview(project, settings, 'level-kit-obj', new Set()).notes;
    expect(notes.some(n => n.includes('are not .obj'))).toBe(true);
  });

  const NEW = [
    ['web-game', 'jpg', 1024, false, ['props', 'characters', 'kit']],
    ['godot', 'png', 2048, false, ['props', 'characters', 'environment']],
    ['unity', 'png', 2048, false, ['props', 'characters', 'kit']],
    ['unreal', 'png', 4096, false, ['props', 'characters', 'environment']],
    ['mobile', 'jpg', 1024, false, ['props', 'characters', 'kit']],
    ['vr-standalone', 'jpg', 1024, false, ['props', 'characters', 'kit']],
    ['game-jam', 'jpg', 1024, true, ['']],
  ] as const;

  it.each(NEW)(
    '%s previews its export settings and applies what it previews',
    (id, textures, cap, drop, folders) => {
      const preview = templatePreview(project, settings, id, new Set());
      expect(preview.formatAfter).toBe('glb');
      expect(preview.exportBefore).toEqual(settings.export);
      expect(preview.exportAfter).toEqual({ textures, textureCap: cap, dropMaps: drop });
      expect(preview.template.budgets.map(b => b.folder)).toEqual(folders);
      expect(preview.template.why).not.toMatch(/[\u2013\u2014]/);
      const { next } = templateOutcome(project, settings, id, new Set());
      expect(next.export).toEqual(preview.exportAfter);
      expect(next.format).toBe('glb');
    }
  );

  it('leaves the export settings alone for the shape templates', () => {
    const preview = templatePreview(project, settings, 'level-kit', new Set());
    expect(preview.exportAfter).toEqual(preview.exportBefore);
  });

  it('notes the dropped maps and the top level row of a game jam', () => {
    const notes = templatePreview(project, settings, 'game-jam', new Set()).notes;
    expect(notes.some(n => n.includes('metal-roughness'))).toBe(true);
    expect(notes.some(n => n.includes('top level'))).toBe(true);
  });
});
