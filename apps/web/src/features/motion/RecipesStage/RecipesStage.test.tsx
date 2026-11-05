import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import RecipesStage from './RecipesStage';

afterEach(() => {
  clearMocks();
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
});

describe('the recipes stage', () => {
  it('says there are no recipes yet and opens the base clip picker', async () => {
    await openMockLibrary();
    render(<RecipesStage />);
    expect(await screen.findByText('No recipes yet')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'New recipe' }));
    expect(useRecipesStore.getState().picking).toBe('frostveil');
  });

  it('lists the project recipes with a Mixamo mark and Apply waiting for baking', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    expect(reach).toHaveTextContent('1 change · 1 open blank');
    expect(reach).toHaveTextContent('Mixamo');
    expect(within(reach).getByRole('button', { name: /Apply/ })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByRole('button', { name: 'New recipe' })).toBeInTheDocument();
  });
});
