import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import RecipeReview from './RecipeReview';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

const LANTERN = 'reach-for-the-lantern';

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
  useRecipeBakesStore.setState(useRecipeBakesStore.getInitialState(), true);
  useMotionStore.setState(useMotionStore.getInitialState(), true);
  useAgentsStore.setState({ status: null });
});

/** The demo lantern recipe's finished bake on the clay figure, open in the review. */
async function openReview() {
  const args = Array<string | undefined>(12).fill(undefined);
  args[0] = 'configured';
  args[6] = 'demo';
  args[11] = 'demo';
  await openMockWorkspace(...(args as Parameters<typeof openMockWorkspace>));
  await useMotionStore.getState().load('frostveil');
  await useRecipesStore.getState().load('frostveil');
  useRecipeBakesStore.getState().openReview('frostveil', LANTERN);
  await waitFor(() =>
    expect(useRecipeBakesStore.getState().bakes[`frostveil/${LANTERN}`]?.state).toBe('done')
  );
  return render(<RecipeReview />);
}

describe('the recipe review', () => {
  it('shows the bake next to the base clip with each number against its limit', async () => {
    await openReview();
    expect(screen.getByRole('heading', { name: 'Review Reach for the lantern' })).toBeVisible();
    expect(screen.getByRole('figure', { name: 'Baked clip' })).toBeInTheDocument();
    expect(screen.getByRole('figure', { name: 'Base clip' })).toBeInTheDocument();
    const numbers = screen.getByRole('list', { name: 'Review numbers' });
    const feet = within(numbers).getByText('Foot slide').closest('li')!;
    expect(feet).toHaveTextContent('2.6 cm (leftFoot)');
    expect(feet).toHaveTextContent('limit 2.3 cm');
    expect(within(feet).getByRole('img', { name: 'Worth a look' })).toBeInTheDocument();
    const gap = within(numbers).getByText('Hand gap').closest('li')!;
    expect(within(gap).getByRole('img', { name: 'Passes' })).toBeInTheDocument();
    expect(screen.getByText('1 of 4 numbers warns')).toBeVisible();

    // Only the bake, then only the base clip.
    await userEvent.click(screen.getByRole('button', { name: 'Bake' }));
    expect(screen.queryByRole('figure', { name: 'Base clip' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Base clip' }));
    expect(screen.queryByRole('figure', { name: 'Baked clip' })).toBeNull();
  });

  it('adds the bake as the next version and goes back to the list', async () => {
    await openReview();
    const add = screen.getByRole('button', { name: 'Add as v3' });
    await userEvent.click(add);
    await waitFor(() => expect(useRecipeBakesStore.getState().reviewing).toBeNull());
    const record = useRecipeBakesStore.getState().bakes[`frostveil/${LANTERN}`];
    expect(record).toMatchObject({ state: 'added', addedVersion: 3 });
    expect(useAppStore.getState().notice?.title).toBe('Added as v3 of Clay figure.');
    const project = await commands.projectGet('frostveil');
    const figure = project.assets?.find(a => a.id === 'clay-figure');
    expect(figure?.versions.at(-1)?.version).toBe(3);
  });

  it('refuses a bake out of date and offers to bake it again', async () => {
    await openReview();
    const recipe = await commands.recipeGet('frostveil', LANTERN);
    await commands.recipeRename('frostveil', LANTERN, 'Reach for the lamp');
    await useRecipesStore.getState().load('frostveil');
    expect(recipe.version).not.toBe(useRecipesStore.getState().list?.[0]?.version);
    const banner = await screen.findByText('This bake is out of date');
    expect(banner).toBeVisible();
    expect(screen.getByText('The recipe changed since this bake. Bake it again.')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Add as v3' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Bake again' }));
    await waitFor(() => expect(useRecipeBakesStore.getState().reviewing).toBeNull());
    expect(useRecipeBakesStore.getState().bakes[`frostveil/${LANTERN}`]?.state).toBe('running');
  });

  it('says when an agent asked to add the bake', async () => {
    await openReview();
    useAgentsStore.setState({
      status: {
        ...(await commands.mcpStatus()),
        approvals: [
          {
            id: 'recipe-approval-1',
            kind: 'recipe-bake',
            project: 'frostveil',
            runId: null,
            runNumber: null,
            step: null,
            job: LANTERN,
            assetName: 'Clay figure v3',
            service: null,
            detail: null,
            items: [],
            requestedAtMs: 0,
          },
        ],
      },
    });
    expect(await screen.findByText(/An agent asked to add this bake/)).toBeVisible();
  });
});
