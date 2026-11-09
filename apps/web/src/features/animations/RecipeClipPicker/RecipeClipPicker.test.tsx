import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { openRecipe } from '@/test/open-recipe';
import RecipeClipPicker from './RecipeClipPicker';

afterEach(() => {
  clearMocks();
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
  useAnimationsStore.setState({ view: null, error: null });
});

describe('the recipe clip picker', () => {
  it('stays closed until the recipes tab opens it', () => {
    const { container } = render(<RecipeClipPicker />);
    expect(container).toBeEmptyDOMElement();
  });

  it('picks a library clip, names the recipe after it and makes the recipe', async () => {
    await openMockLibrary('demo');
    await useRecipesStore.getState().load('frostveil');
    const before = useRecipesStore.getState().list!.length;
    useRecipesStore.getState().openPicker('frostveil');
    render(<RecipeClipPicker />);
    const sheet = await screen.findByRole('dialog', { name: 'New recipe' });
    const make = within(sheet).getByRole('button', { name: /Make recipe/ });
    expect(make).toHaveAttribute('aria-disabled', 'true');
    const list = await within(sheet).findByRole('list', { name: 'Library clips' });
    const first = within(list).getAllByRole('button')[0]!;
    const name = first.getAttribute('aria-label')!;
    await userEvent.click(first);
    expect(within(sheet).getByLabelText('Recipe name')).toHaveValue(name);
    expect(sheet).toHaveTextContent('counts as Mixamo');
    await userEvent.click(make);
    expect(useRecipesStore.getState().picking).toBeNull();
    expect(useRecipesStore.getState().list).toHaveLength(before + 1);
  });

  it('narrows the clips with a search', async () => {
    await openMockLibrary('demo');
    useRecipesStore.getState().openPicker('frostveil');
    render(<RecipeClipPicker />);
    const sheet = await screen.findByRole('dialog', { name: 'New recipe' });
    const list = await within(sheet).findByRole('list', { name: 'Library clips' });
    const all = within(list).getAllByRole('button').length;
    await userEvent.type(within(sheet).getByLabelText('Search clips'), 'zzz nothing');
    expect(within(list).queryAllByRole('button')).toHaveLength(0);
    expect(sheet).toHaveTextContent('No clip matches.');
    expect(all).toBeGreaterThan(0);
  });

  it('makes a recipe on the rest pose, named after it until the person types a name', async () => {
    await openMockLibrary('demo');
    await useRecipesStore.getState().load('frostveil');
    useRecipesStore.getState().openPicker('frostveil');
    render(<RecipeClipPicker />);
    const sheet = await screen.findByRole('dialog', { name: 'New recipe' });
    const rest = within(sheet).getByRole('button', { name: 'Rest pose' });
    await userEvent.click(rest);
    expect(rest).toHaveAttribute('aria-pressed', 'true');
    expect(within(sheet).getByLabelText('Recipe name')).toHaveValue('Rest pose');
    expect(sheet).toHaveTextContent('On the rest pose');
    expect(sheet).not.toHaveTextContent('counts as Mixamo');
    await userEvent.clear(within(sheet).getByLabelText('Recipe name'));
    await userEvent.type(within(sheet).getByLabelText('Recipe name'), 'Stand and wave');
    await userEvent.click(within(sheet).getByRole('button', { name: /Make recipe/ }));
    const made = useRecipesStore.getState().list!.find(r => r.name === 'Stand and wave');
    expect(made?.base).toBe('Rest pose');
  });

  it('joins a library clip to the recipe open in the editor', async () => {
    await openRecipe('slash-into-block');
    useRecipesStore.getState().openPicker('frostveil', true);
    render(<RecipeClipPicker />);
    const sheet = await screen.findByRole('dialog', { name: 'Add a clip' });
    expect(within(sheet).queryByRole('button', { name: 'Rest pose' })).not.toBeInTheDocument();
    expect(within(sheet).queryByLabelText('Recipe name')).not.toBeInTheDocument();
    const list = await within(sheet).findByRole('list', { name: 'Library clips' });
    const first = within(list).getAllByRole('button')[0]!;
    await userEvent.click(first);
    await userEvent.click(within(sheet).getByRole('button', { name: /Add clip/ }));
    const draft = useRecipeEditorStore.getState().editing!.draft;
    expect(draft.clips).toHaveLength(4);
    expect(draft.clips[3]?.name).toBe(first.getAttribute('aria-label'));
    expect(draft.kept[3]).toBeNull();
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'clips', id: '' });
    expect(useRecipesStore.getState()).toMatchObject({ picking: null, joining: false });
  });
});
