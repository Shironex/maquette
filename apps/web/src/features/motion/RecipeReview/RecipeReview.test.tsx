import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import RecipeReview from './RecipeReview';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));
vi.mock('@/features/viewer/pose-snapshots', () => ({
  poseStills: () => Promise.reject(new Error('no WebGL in jsdom')),
}));

const LANTERN = 'reach-for-the-lantern';
const JOINED = 'slash-into-block';

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
  useRecipeBakesStore.setState(useRecipeBakesStore.getInitialState(), true);
  useMotionStore.setState(useMotionStore.getInitialState(), true);
  useRecipeEditorStore.setState(useRecipeEditorStore.getInitialState(), true);
  useAgentsStore.setState({ status: null });
});

/** The demo lantern recipe's finished bake on the clay figure, open in the review. */
async function openReview(recipe = LANTERN) {
  const args = Array<string | undefined>(12).fill(undefined);
  args[0] = 'configured';
  args[6] = 'demo';
  args[11] = 'demo';
  await openMockWorkspace(...(args as Parameters<typeof openMockWorkspace>));
  await useMotionStore.getState().load('frostveil');
  await useRecipesStore.getState().load('frostveil');
  if (recipe !== LANTERN) {
    // Only the lantern has a finished bake in the demo: bake this one first.
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const doc = await commands.recipeGet('frostveil', recipe);
    await commands.recipeBakeStart({
      project: 'frostveil',
      recipe,
      asset: 'clay-figure',
      expected: doc.version,
    });
    await vi.advanceTimersByTimeAsync(10_000);
    vi.useRealTimers();
  }
  useRecipeBakesStore.getState().openReview('frostveil', recipe);
  await waitFor(() =>
    expect(useRecipeBakesStore.getState().bakes[`frostveil/${recipe}`]?.state).toBe('done')
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

  it('shows a seam pop with its pose sheet and lays the best fix on the recipe', async () => {
    await openReview(JOINED);
    const numbers = screen.getByRole('list', { name: 'Review numbers' });
    const seam = within(numbers).getByText('Seam pop').closest('li')!;
    expect(within(seam).getByRole('img', { name: 'Worth a look' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'Worth a look: Seam pop (Sword And Shield Slash into Sword And Shield Block), Foot slide. It can still be added.'
      )
    ).toBeVisible();
    const sheet = screen.getByRole('region', { name: 'Pose sheet' });
    const tile = within(sheet).getByRole('button', { name: /seam 1/ });
    await userEvent.click(tile);
    expect(tile).toHaveAttribute('aria-pressed', 'true');

    const fixes = within(screen.getByRole('region', { name: 'Fixes' })).getAllByRole('listitem');
    expect(fixes[0]).toHaveTextContent(/Widen the blend to (\d+) f/);
    const frames = Number(/Widen the blend to (\d+) f/.exec(fixes[0]!.textContent ?? '')?.[1]);
    await userEvent.click(within(fixes[0]!).getByRole('button', { name: 'Try it' }));
    await waitFor(() => expect(useRecipeBakesStore.getState().reviewing).toBeNull());
    const editing = useRecipeEditorStore.getState().editing;
    expect(editing?.id).toBe(JOINED);
    expect(editing?.draft.clips[1]?.blend?.frames).toBe(frames);
  });
});
