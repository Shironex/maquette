import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeLanes from './RecipeLanes';

afterEach(clearMocks);

describe('the recipe lanes', () => {
  it('lays the clips, trims, retime, freeze and mirror over the bake frames', async () => {
    await openRecipe('slash-into-block');
    render(<RecipeLanes frame={0} playing={false} onToggle={vi.fn()} onScrub={vi.fn()} />);
    expect(
      screen.getByRole('button', { name: /^Clip 2, Sword And Shield Block 2 to 28, Hash differs/ })
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Retime 1.6x' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Freeze 12 f' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Mirror off' })).toBeVisible();
    expect(screen.getByText(/^0 \/ \d+ f · 0\.00 \/ /)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Freeze 12 f' }));
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'time', id: '1' });
  });

  it('scrubs and plays from its own controls', async () => {
    await openRecipe();
    const onToggle = vi.fn();
    render(<RecipeLanes frame={0} playing onToggle={onToggle} onScrub={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(onToggle).toHaveBeenCalled();
    expect(screen.getByRole('button', { name: /^Reach l-1, frames/ })).toBeVisible();
  });
});

describe('the pin lane', () => {
  it('lays pins on their own lane, stacking the ones that overlap', async () => {
    await openRecipe('slash-into-block');
    const pin = (id: string, bone: string) => ({
      id,
      kind: 'pin' as const,
      bone,
      settings: {
        weight: 1,
        start: 0.2,
        end: 0.6,
        target: null,
        rotation: null,
        offset: null,
        cycles: null,
        amount: null,
      },
    });
    useRecipeEditorStore.getState().edit(d => ({
      ...d,
      layers: [...d.layers, pin('l-8', 'leftFoot'), pin('l-9', 'rightFoot')],
    }));
    render(<RecipeLanes frame={0} playing={false} onToggle={vi.fn()} onScrub={vi.fn()} />);
    expect(screen.getByText('Pins')).toBeVisible();
    const left = screen.getByRole('button', { name: /^Pin l-8, frames/ });
    expect(left).toHaveTextContent('Left foot');
    expect(screen.getByRole('button', { name: /^Pin l-9, frames/ })).toHaveTextContent(
      'Right foot'
    );
    await userEvent.click(left);
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'layer', id: 'l-8' });
  });
});

describe('the grip lane', () => {
  it('names the library grip and revision a grip was copied from', async () => {
    await openRecipe('scythe-charge-held');
    render(<RecipeLanes frame={0} playing={false} onToggle={vi.fn()} onScrub={vi.fn()} />);
    const bar = await screen.findByRole('button', { name: /^Grip l-2, frames/ });
    expect(await screen.findByText('Scythe, staff grip r1')).toBeVisible();
    await userEvent.click(bar);
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'layer', id: 'l-2' });
  });
});
