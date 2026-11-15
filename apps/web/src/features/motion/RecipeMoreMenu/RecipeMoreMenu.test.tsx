import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { removeBlocked, removeItem } from './RecipeMoreMenu.hooks';
import RecipeMoreMenu from './RecipeMoreMenu';

describe('the recipe More menu', () => {
  it('opens on the More button and runs Remove recipe', async () => {
    const onRemove = vi.fn();
    render(<RecipeMoreMenu name="Swing" items={[removeItem(onRemove, removeBlocked(false))]} />);
    await userEvent.click(screen.getByRole('button', { name: 'More for Swing' }));
    await userEvent.click(screen.getByRole('menuitem', { name: /^Remove recipe/ }));
    expect(onRemove).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('says why Remove recipe is off while it bakes or has unsaved edits', () => {
    expect(removeBlocked(true)).toMatch(/baking/);
    expect(removeBlocked(false, true)).toMatch(/Save or discard/);
    expect(removeBlocked(false)).toBeNull();
    const off = removeItem(() => undefined, 'It is baking.');
    expect(off).toMatchObject({ disabled: true, hint: 'It is baking.', tone: 'danger' });
  });
});
