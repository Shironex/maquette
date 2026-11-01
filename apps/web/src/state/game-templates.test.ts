import { describe, expect, it } from 'vitest';
import { GAME_TEMPLATES } from '@/ipc/mock/game-templates';
import { budgetsWords, exportWords, templateSections } from './game-templates';

const get = (id: string) => GAME_TEMPLATES.find(t => t.id === id)!;

describe('game template words', () => {
  it('groups every template once, shapes first', () => {
    const sections = templateSections(GAME_TEMPLATES);
    expect(sections.map(s => s.id)).toEqual(['shape', 'engine', 'target']);
    expect(sections.flatMap(s => s.templates.map(t => t.id)).sort()).toEqual(
      GAME_TEMPLATES.map(t => t.id).sort()
    );
    expect(sections[1]!.templates.map(t => t.name)).toEqual(['Godot', 'Unity', 'Unreal']);
  });

  it('writes the numbers a card shows', () => {
    expect(exportWords(get('web-game'))).toBe('.glb, JPG up to 1024 px');
    expect(budgetsWords(get('web-game'))).toBe('props/ 10k, characters/ 25k, kit/ 3k');
    expect(exportWords(get('game-jam'))).toBe('.glb, JPG up to 1024 px, no extra maps');
    expect(budgetsWords(get('game-jam'))).toBe('all folders 20k');
    expect(exportWords(get('level-kit-obj'))).toBe('.obj');
    expect(budgetsWords(get('unreal'))).toBe('props/ 40k, characters/ 80k, environment/ 60k');
  });
});
