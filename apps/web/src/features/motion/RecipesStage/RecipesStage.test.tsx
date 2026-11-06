import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { act } from 'react';
import { useAgentsStore } from '@/state/use-agents-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { useRecipeCompareStore } from '@/state/use-recipe-compare-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { useRecipeSaveStore } from '@/state/use-recipe-save-store';
import { clearMocks, openMockLibrary, openMockWorkspace } from '@/test/mock-ipc';
import RecipesStage from './RecipesStage';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => {
  clearMocks();
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
  useRecipeEditorStore.setState(useRecipeEditorStore.getInitialState(), true);
  useRecipeBakesStore.setState(useRecipeBakesStore.getInitialState(), true);
  useMotionStore.setState(useMotionStore.getInitialState(), true);
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
  useRecipeSaveStore.setState(useRecipeSaveStore.getInitialState(), true);
  useRecipeCompareStore.setState(useRecipeCompareStore.getInitialState(), true);
});

/** The demo recipes with Frostveil's rigged demo models loaded, as the Motion view has them. */
async function openWithModels() {
  const args = Array<string | undefined>(12).fill(undefined);
  args[0] = 'configured';
  args[6] = 'demo';
  args[11] = 'demo';
  await openMockWorkspace(...(args as Parameters<typeof openMockWorkspace>));
  await useMotionStore.getState().load('frostveil');
}

describe('the recipes stage', () => {
  it('leaves room under the last row for a floating approval card', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    expect(list).not.toHaveClass('pb-72');
    const status = { approvals: [{ id: 'a' }] } as unknown as NonNullable<
      ReturnType<typeof useAgentsStore.getState>['status']
    >;
    act(() => useAgentsStore.setState({ status }));
    expect(list).toHaveClass('pb-72');
    act(() => useAgentsStore.setState(useAgentsStore.getInitialState(), true));
  });

  it('says there are no recipes yet and opens the base clip picker', async () => {
    await openMockLibrary();
    render(<RecipesStage />);
    expect(await screen.findByText('No recipes yet')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'New recipe' }));
    expect(useRecipesStore.getState().picking).toBe('frostveil');
  });

  it('lists the project recipes with a Mixamo mark and Apply waiting for a rigged model', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    expect(reach).toHaveTextContent('3 changes');
    expect(reach).not.toHaveTextContent('open blank');
    expect(reach).toHaveTextContent('Mixamo');
    expect(within(reach).getByRole('button', { name: /Apply/ })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByRole('button', { name: 'New recipe' })).toBeInTheDocument();
  });

  it('says when a prop blank reads open because its asset is gone', async () => {
    await openMockLibrary('demo');
    const warning =
      'The blank lantern names lantern, which is no longer an asset of Frostveil, so it reads as open.';
    vi.spyOn(commands, 'recipeList').mockResolvedValueOnce([
      {
        id: 'reach',
        file: 'recipes/reach.recipe.json',
        name: 'Reach for the lantern',
        base: 'Standing Idle',
        frames: 120,
        mixamo: false,
        layers: 1,
        blanks: 1,
        open: 1,
        edited: '2026-11-05T10:00:00.000Z',
        version: 'v1',
        motionVersion: 'm1',
        warnings: [warning],
        problem: null,
        from: null,
      },
    ]);
    render(<RecipesStage />);
    expect(await screen.findByText(warning)).toBeInTheDocument();
  });

  it('reads a gone prop as an open blank with its warning, from the demo', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const look = within(list).getByText('Look around').closest('li')!;
    expect(look).toHaveTextContent('1 open blank');
    expect(look).toHaveTextContent(/The blank torch names old-torch, which is no longer an asset/);
  });

  it('opens a recipe in the editor and goes back to the list', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    await userEvent.click(within(reach).getByRole('button', { name: 'Edit' }));
    expect(
      await screen.findByRole('region', { name: 'Recipe Reach for the lantern' })
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    expect(await screen.findByRole('list', { name: 'Project recipes' })).toBeVisible();
  });

  it('applies a recipe onto the rigged model, shows its progress and cancels it', async () => {
    await openWithModels();
    const start = vi.spyOn(commands, 'recipeBakeStart');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    // The demo opens with a finished bake of this recipe.
    expect(await within(reach).findByRole('button', { name: 'Review' })).toBeVisible();
    expect(within(reach).queryByText('Out of date')).not.toBeInTheDocument();
    await userEvent.click(within(reach).getByRole('button', { name: 'Apply' }));
    expect(await within(reach).findByRole('progressbar')).toBeVisible();
    expect(start.mock.calls[0]?.[0]).toMatchObject({
      project: 'frostveil',
      recipe: 'reach-for-the-lantern',
      expected: expect.stringMatching(/.+/),
      asset: useMotionStore.getState().view?.models.find(m => m.rig && !m.error)?.asset,
    });
    await userEvent.click(within(reach).getByRole('button', { name: 'Cancel' }));
    expect(await within(reach).findByText(/last bake was cancelled/)).toBeVisible();
  });

  it('opens a finished bake in the review and goes back to the list', async () => {
    await openWithModels();
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    await userEvent.click(await within(reach).findByRole('button', { name: 'Review' }));
    expect(await screen.findByRole('region', { name: 'Recipe review' })).toBeVisible();
    expect(screen.queryByRole('list', { name: 'Project recipes' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(await screen.findByRole('list', { name: 'Project recipes' })).toBeVisible();
  });

  it('refuses a bake of a recipe that changed since the list read it', async () => {
    await openWithModels();
    const listed = await commands.recipeList('frostveil');
    vi.spyOn(commands, 'recipeList').mockResolvedValueOnce(
      listed.map(r =>
        r.id === 'reach-for-the-lantern' ? { ...r, version: 'older', motionVersion: 'older' } : r
      )
    );
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    // Its finished bake reads as out of date against the version listed.
    expect(await within(reach).findByText('Out of date')).toBeVisible();
    await userEvent.click(within(reach).getByRole('button', { name: 'Apply' }));
    expect(await within(reach).findByText(/changed since it was read/)).toBeVisible();
    expect(within(reach).queryByRole('progressbar')).not.toBeInTheDocument();
  });

  it('reads a bake record from before the motion version as out of date once the file changed', async () => {
    await openWithModels();
    const listed = await commands.recipeList('frostveil');
    const reach = listed.find(r => r.id === 'reach-for-the-lantern')!;
    const real = await commands.recipeBakeGet('frostveil', 'reach-for-the-lantern');
    expect(real?.result).toBeTruthy();
    // An old record holds only the file version, which the recipe does not have.
    const old = {
      ...real!,
      stale: null,
      result: { ...real!.result!, motionVersion: '', recipeVersion: 'older' },
    };
    vi.spyOn(commands, 'recipeBakeGet').mockResolvedValue(old);
    vi.spyOn(commands, 'recipeList').mockResolvedValueOnce(
      listed.map(r => (r.id === reach.id ? { ...r, version: 'newer' } : r))
    );
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const row = within(list).getByText('Reach for the lantern').closest('li')!;
    expect(await within(row).findByText('Out of date')).toBeVisible();
  });

  it('keeps Apply off for a recipe with an open blank', async () => {
    await openWithModels();
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const look = within(list).getByText('Look around').closest('li')!;
    expect(within(look).getByRole('button', { name: /Apply/ })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});

describe('the recipes stage and the recipe library', () => {
  /** The lantern recipe's row in the demo list. */
  async function lanternRow() {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    return within(list).getByText('Reach for the lantern').closest('li')!;
  }

  it('says the library has a newer revision and opens What changed', async () => {
    const reach = await lanternRow();
    expect(await within(reach).findByText('Library has r3')).toBeVisible();
    await userEvent.click(within(reach).getByRole('button', { name: 'What changed' }));
    expect(useRecipeCompareStore.getState().sheet).toMatchObject({
      project: 'frostveil',
      recipe: 'reach-for-the-lantern',
    });
  });

  it('says quietly when the library recipe it came from is gone', async () => {
    await openMockLibrary('demo');
    await commands.recipeLibraryRemove('reach-for-the-lantern', null);
    render(<RecipesStage />);
    expect(await screen.findByText('Its library recipe is gone.')).toBeVisible();
    expect(screen.queryByText(/^Library has/)).toBeNull();
  });

  it('saves to the library and exports from the row menu', async () => {
    const reach = await lanternRow();
    const exported = vi.spyOn(commands, 'recipeExport');
    await userEvent.click(
      within(reach).getByRole('button', { name: 'More for Reach for the lantern' })
    );
    await userEvent.click(screen.getByRole('menuitem', { name: /^Save to library/ }));
    expect(useRecipeSaveStore.getState().sheet).toMatchObject({
      project: 'frostveil',
      recipe: 'reach-for-the-lantern',
    });
    await userEvent.click(
      within(reach).getByRole('button', { name: 'More for Reach for the lantern' })
    );
    await userEvent.click(screen.getByRole('menuitem', { name: /^Export/ }));
    expect(exported).toHaveBeenCalledWith(
      { project: 'frostveil', recipe: 'reach-for-the-lantern', entry: null, revision: null },
      'Reach for the lantern'
    );
  });

  it('imports a recipe and shows what to know under the header', async () => {
    await lanternRow();
    await userEvent.click(screen.getByRole('button', { name: 'Import' }));
    const notes = await screen.findByRole('list', { name: 'Import notes' });
    expect(screen.getByText('Imported Sit and wave')).toBeVisible();
    expect(notes).toHaveTextContent(/hash differs/);
    expect(notes).toHaveTextContent(/stone-bench, which Frostveil does not have/);
    expect(await screen.findByText('Sit and wave')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByRole('list', { name: 'Import notes' })).toBeNull();
  });
});
