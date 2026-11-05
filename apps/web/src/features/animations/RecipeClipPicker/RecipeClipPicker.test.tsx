import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
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
});
