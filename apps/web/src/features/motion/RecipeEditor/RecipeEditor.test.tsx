import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type RecipeLayer } from '@/ipc/client';
import { bakeModelOf } from '@/state/recipe-bake';
import type { RecipeDraft } from '@/state/recipe-draft';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeEditor from './RecipeEditor';

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
  useRecipeBakesStore.setState(useRecipeBakesStore.getInitialState(), true);
});

const weigh = (weight: number) => (l: RecipeLayer[]) =>
  l.map(x => (x.id === 'l-1' ? { ...x, settings: { ...x.settings, weight } } : x));
const edit = (weight: number) =>
  act(() => useRecipeEditorStore.getState().edit(d => ({ ...d, layers: weigh(weight)(d.layers) })));

/** An agent adds a grip over MCP. */
async function addGrip(id: string) {
  const doc = await commands.recipeGet('frostveil', id);
  const keys: RecipeLayer = {
    id: 'l-9',
    kind: 'grip',
    bone: 'rightHand',
    settings: { ...doc.recipe.layers[0]!.settings, target: null, start: 0, end: 1 },
    mode: 'replace',
    pose: { rightIndexProximal: [0, 0, 30] },
  };
  await commands.recipeUpdate('frostveil', id, {
    name: null,
    layers: [...doc.recipe.layers, keys],
    blanks: null,
    expected: doc.version,
  });
}

describe('the recipe editor', () => {
  it('saves only with unsaved edits, then says saved', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    const save = () => screen.getByRole('button', { name: 'Save' });
    expect(save()).toHaveAttribute('aria-disabled', 'true');
    // A Save with nothing to save looks as the other disabled buttons do, not primary.
    expect(save()).not.toHaveClass('bg-clay');
    edit(0.5);
    expect(screen.getByText(/unsaved edits$/)).toBeVisible();
    expect(save()).toHaveClass('bg-clay');
    await userEvent.click(save());
    expect(await screen.findByText(/· saved$/)).toBeVisible();
    expect(save()).not.toHaveClass('bg-clay');
  });

  it('asks before Back drops unsaved edits, and stays when asked to', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    edit(0.5);
    await userEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    const dialog = screen.getByRole('dialog', { name: 'Save the recipe first?' });
    expect(dialog).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Stay' }));
    expect(useRecipeEditorStore.getState().editing?.draft.layers[0]?.settings.weight).toBe(0.5);
    await userEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    await userEvent.click(screen.getByRole('button', { name: 'Leave without saving' }));
    expect(useRecipeEditorStore.getState().editing).toBeNull();
  });

  it('asks Keep mine or Use theirs when an agent changed what the person edited', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    edit(0.5);
    const doc = await commands.recipeGet('frostveil', 'reach-for-the-lantern');
    await commands.recipeUpdate('frostveil', 'reach-for-the-lantern', {
      name: null,
      layers: weigh(0.9)(doc.recipe.layers),
      blanks: null,
    });
    await act(() => useRecipeEditorStore.getState().onChanged('frostveil'));
    expect(screen.getByRole('dialog', { name: 'The recipe changed outside' })).toHaveTextContent(
      'Both sides changed change l-1.'
    );
    await userEvent.keyboard('{Escape}');
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Answer clash' }));
    await userEvent.click(screen.getByRole('button', { name: 'Keep mine' }));
    expect(useRecipeEditorStore.getState().editing?.draft.layers[0]?.settings.weight).toBe(0.5);
  });

  it('says what to fix before Save', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    act(() =>
      useRecipeEditorStore.getState().edit(d => ({
        ...d,
        layers: d.layers.map(l =>
          l.id === 'l-1' ? { ...l, settings: { ...l.settings, target: null } } : l
        ),
      }))
    );
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByText('A reach needs a target: pick a blank, or add one.')).toBeVisible();
  });

  it('blocks Save and Apply while a prop blank holds the model the recipe animates', async () => {
    await openRecipe();
    const { view, asset } = useMotionStore.getState();
    const model = bakeModelOf(view, asset)!;
    const holdModel = (d: RecipeDraft) => ({
      ...d,
      blanks: d.blanks.map(b => (b.id === 'lantern' ? { ...b, prop: model.asset } : b)),
    });
    const { unmount } = render(<RecipeEditor />);
    act(() => useRecipeEditorStore.getState().edit(holdModel));
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('button', { name: 'Save and apply' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    unmount();
    // The same held in the file itself: Apply waits too, with no unsaved edits.
    const doc = useRecipeEditorStore.getState().editing!.doc;
    await commands.recipeUpdate('frostveil', doc.id, {
      name: null,
      layers: null,
      blanks: holdModel(useRecipeEditorStore.getState().editing!.draft).blanks,
    });
    useRecipeEditorStore.setState(useRecipeEditorStore.getInitialState(), true);
    await useRecipeEditorStore.getState().open('frostveil', doc.id);
    const start = vi.spyOn(commands, 'recipeBakeStart');
    render(<RecipeEditor />);
    const apply = screen.getByRole('button', { name: 'Apply' });
    expect(apply).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(apply);
    expect(start).not.toHaveBeenCalled();
  });

  it('saves unsaved edits before it applies, and bakes the version it saved', async () => {
    await openRecipe();
    const start = vi.spyOn(commands, 'recipeBakeStart');
    render(<RecipeEditor />);
    edit(0.5);
    await userEvent.click(screen.getByRole('button', { name: 'Save and apply' }));
    expect(await screen.findByRole('progressbar')).toBeVisible();
    const saved = useRecipeEditorStore.getState().editing!.doc;
    expect(saved.recipe.layers[0]?.settings.weight).toBe(0.5);
    expect(start.mock.calls[0]?.[0]).toMatchObject({
      recipe: 'reach-for-the-lantern',
      expected: saved.version,
    });
    expect(screen.getByRole('button', { name: 'Apply' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  });

  it('does not bake when the save cannot go through', async () => {
    await openRecipe();
    const start = vi.spyOn(commands, 'recipeBakeStart');
    render(<RecipeEditor />);
    act(() =>
      useRecipeEditorStore.getState().edit(d => ({
        ...d,
        layers: d.layers.map(l =>
          l.id === 'l-1' ? { ...l, settings: { ...l.settings, target: null } } : l
        ),
      }))
    );
    expect(screen.getByRole('button', { name: 'Save and apply' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(start).not.toHaveBeenCalled();
  });

  it('edits a joined recipe, with its base edits and the bake length in the header', async () => {
    await openRecipe('slash-into-block');
    render(<RecipeEditor />);
    expect(screen.getByRole('region', { name: 'Base edits' })).toBeInTheDocument();
    expect(
      screen.getByText(
        /^Sword And Shield Slash \+ Sword And Shield Block \+ Sword And Shield Idle · \d+ f · /
      )
    ).toBeInTheDocument();
    expect(
      screen.queryByText('This recipe uses what this Maquette cannot edit or bake yet')
    ).toBeNull();
  });

  it('edits and saves a recipe holding a grip, with Save and Apply', async () => {
    await openRecipe('scythe-charge-held');
    render(<RecipeEditor />);
    expect(screen.getByRole('region', { name: 'Changes' })).toHaveTextContent(
      'Grip · Both hands · scythe-staff-grip r1'
    );
    expect(screen.getByRole('button', { name: /apply/i })).toBeVisible();
    act(() =>
      useRecipeEditorStore.getState().edit(d => ({
        ...d,
        layers: d.layers.map(l =>
          l.kind === 'grip' ? { ...l, settings: { ...l.settings, weight: 0.5 } } : l
        ),
      }))
    );
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    const saved = await commands.recipeGet('frostveil', 'scythe-charge-held');
    expect(saved.recipe.layers.find(l => l.kind === 'grip')?.settings.weight).toBe(0.5);
    expect(screen.getByText(/ · saved$/)).toBeInTheDocument();
  });

  it('merges a grip an agent adds into unsaved edits, and saves both', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    edit(0.5);
    const id = useRecipeEditorStore.getState().editing!.id;
    await addGrip(id);
    await act(() => useRecipeEditorStore.getState().onChanged('frostveil'));
    expect(useRecipeEditorStore.getState().editing?.draft.layers.at(-1)?.kind).toBe('grip');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    const saved = await commands.recipeGet('frostveil', id);
    expect(saved.recipe.layers.some(l => l.kind === 'grip')).toBe(true);
    expect(saved.recipe.layers[0]?.settings.weight).toBe(0.5);
  });

  it('shows Add a grip in place of the preview and the changes until it closes', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    act(() => useRecipeEditorStore.getState().pick({ what: 'grips', id: '' }));
    expect(screen.getByRole('region', { name: 'Add a grip' })).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Changes' })).toBeNull();
    await userEvent.click(await screen.findByRole('button', { name: 'Close' }));
    expect(screen.getByRole('region', { name: 'Changes' })).toBeVisible();
  });

  it('goes back to version 1 once nothing new is used', async () => {
    await openRecipe();
    const doc = useRecipeEditorStore.getState().editing!.doc;
    const on = await commands.recipeUpdate('frostveil', doc.id, {
      name: null,
      layers: null,
      blanks: null,
      mirror: true,
    });
    expect(on.recipe.version).toBe(2);
    const off = await commands.recipeUpdate('frostveil', doc.id, {
      name: null,
      layers: null,
      blanks: null,
      mirror: false,
    });
    expect(off.recipe.version).toBe(1);
    expect(off.motionVersion).toBe(doc.motionVersion);
  });
});
