import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import RecipeNote from './RecipeNote';

describe('RecipeNote', () => {
  it('shows its title, detail and actions', () => {
    render(
      <RecipeNote
        icon="warning"
        warn
        title="Scythe, staff grip has r2"
        detail="This recipe holds r1."
        actions={<button type="button">Update to r2</button>}
      />
    );
    expect(screen.getByText('Scythe, staff grip has r2')).toBeVisible();
    expect(screen.getByText('This recipe holds r1.')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Update to r2' })).toBeVisible();
  });
});
