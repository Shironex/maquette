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
});
