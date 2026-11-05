import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeBlanks from './RecipeBlanks';

afterEach(clearMocks);

describe('the recipe blanks', () => {
  it('lists each blank with what fills it', async () => {
    await openRecipe();
    render(<RecipeBlanks />);
    const list = screen.getByRole('list', { name: 'Recipe blanks' });
    expect(within(list).getByText('Where the eyes go after').closest('li')).toHaveTextContent(
      '0.35, 1.45, 0.60 m'
    );
  });

  it('shows the warning row for a prop the project no longer has', async () => {
    await openRecipe('look-around');
    render(<RecipeBlanks />);
    expect(screen.getByText('A prop is gone')).toBeVisible();
    expect(screen.getByText(/The blank torch named old-torch/)).toBeVisible();
    expect(screen.getByText('open, pick a prop')).toBeVisible();
  });

  it('adds an open point blank and opens it', async () => {
    await openRecipe();
    render(<RecipeBlanks />);
    await userEvent.click(screen.getByRole('button', { name: 'Add point' }));
    expect(screen.getByRole('group', { name: 'Blank point-1 settings' })).toBeVisible();
    expect(screen.getByText('open, set a point')).toBeVisible();
  });
});
