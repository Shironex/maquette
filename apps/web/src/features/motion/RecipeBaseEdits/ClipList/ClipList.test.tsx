import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import ClipList from './ClipList';

afterEach(clearMocks);

const draft = () => useRecipeEditorStore.getState().editing!.draft;

describe('the recipe clip list', () => {
  it('lists the clips in order with their library state', async () => {
    await openRecipe('slash-into-block');
    render(<ClipList />);
    const items = within(screen.getByRole('list', { name: 'Clips in order' })).getAllByRole(
      'listitem'
    );
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent('Sword And Shield Slash');
    expect(items[1]).toHaveTextContent('Hash differs');
    expect(screen.getAllByRole('group', { name: /Blend into clip/ })).toHaveLength(2);
  });

  it('refuses to take out a clip a time edit sits on, and moves clips with their edits', async () => {
    await openRecipe('slash-into-block');
    render(<ClipList />);
    await userEvent.click(screen.getByRole('button', { name: 'Take Sword And Shield Slash out' }));
    expect(screen.getByRole('alert')).toHaveTextContent('cannot go');
    expect(draft().clips).toHaveLength(3);
    // The nudge runs across the block into the idle, so those two stay together.
    const up = screen.getByRole('button', { name: 'Move Sword And Shield Idle up' });
    await userEvent.click(up);
    expect(screen.getByRole('alert')).toHaveTextContent('cannot move away from clip 2');
    expect(draft().clips.map(c => c.name)[1]).toMatch(/Block/);
    act(() => useRecipeEditorStore.getState().edit(d => ({ ...d, layers: [] })));
    await userEvent.click(up);
    expect(draft().clips.map(c => c.name)[1]).toMatch(/Idle/);
    expect(draft().kept).toEqual([0, 2, 1]);
  });

  it('changes a blend and opens the picker to join a clip', async () => {
    await openRecipe('slash-into-block');
    render(<ClipList />);
    const blend = screen.getByRole('group', { name: 'Blend into clip 2' });
    await userEvent.selectOptions(within(blend).getByLabelText('Line up'), 'off');
    expect(draft().clips[1]?.blend?.lineUp).toBe('off');
    await userEvent.click(screen.getByRole('button', { name: 'Add a clip' }));
    expect(useRecipesStore.getState().joining).toBe(true);
  });
});
