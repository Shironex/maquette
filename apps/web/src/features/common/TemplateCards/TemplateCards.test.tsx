import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { GAME_TEMPLATES } from '@/ipc/mock/game-templates';
import { TemplateCards } from '.';

describe('TemplateCards', () => {
  it('lists every template once, under its section, with its numbers on the card', () => {
    render(
      <TemplateCards
        templates={GAME_TEMPLATES}
        picked={'godot'}
        onPick={vi.fn()}
        columns="grid-cols-2"
      />
    );
    const engines = screen.getByRole('group', { name: 'By engine' });
    expect(
      within(engines)
        .getAllByRole('button')
        .map(b => b.textContent)
    ).toEqual([
      expect.stringMatching(/^Godot/),
      expect.stringMatching(/^Unity/),
      expect.stringMatching(/^Unreal/),
    ]);
    const godot = within(engines).getByRole('button', { name: /^Godot/ });
    expect(godot).toHaveAttribute('aria-pressed', 'true');
    expect(godot).toHaveTextContent('.glb, PNG up to 2048 px');
    expect(godot).toHaveTextContent('props/ 20k, characters/ 50k, environment/ 30k');
    expect(screen.getAllByRole('button')).toHaveLength(GAME_TEMPLATES.length);
  });

  it('hands the picked template back, and shows a leading card first', async () => {
    const onPick = vi.fn();
    render(
      <TemplateCards
        templates={GAME_TEMPLATES}
        picked={null}
        onPick={onPick}
        columns="grid-cols-3"
        leading={<button type="button">No template</button>}
      />
    );
    expect(screen.getAllByRole('button')[0]).toHaveTextContent('No template');
    await userEvent.click(screen.getByRole('button', { name: /^Mobile/ }));
    expect(onPick).toHaveBeenCalledWith(expect.objectContaining({ id: 'mobile' }));
  });
});
