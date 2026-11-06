import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { starterList } from '@/ipc/mock/recipe-starters';
import StarterRecipes from './StarterRecipes';

describe('the starter recipes', () => {
  it('lists each starter read only, with its line and whether it needs a clip', async () => {
    const onSelect = vi.fn();
    render(
      <StarterRecipes starters={starterList([])} selected="starter:lean-in" onSelect={onSelect} />
    );
    const list = screen.getByRole('list', { name: 'Starter recipes' });
    expect(within(list).getAllByRole('button')).toHaveLength(5);
    const wave = within(list).getByRole('button', { name: 'Wave hello, starter' });
    expect(wave).toHaveTextContent('Rest pose · 2 changes');
    expect(wave).toHaveTextContent('Starter');
    expect(wave).not.toHaveTextContent('Needs clip');
    const lean = within(list).getByRole('button', { name: 'Lean in, starter' });
    expect(lean).toHaveAttribute('aria-pressed', 'true');
    expect(lean).toHaveTextContent('Needs clip');
    expect(
      within(list).getByRole('button', { name: 'Reach for a prop, starter' })
    ).toHaveTextContent('Idle · 1 change · 1 blank');
    await userEvent.click(wave);
    expect(onSelect).toHaveBeenCalledWith('starter:wave-hello');
  });
});
