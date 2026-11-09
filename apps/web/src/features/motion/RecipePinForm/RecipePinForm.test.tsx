import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { RecipeLayer } from '@/ipc/client';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipePinChecks } from '@/state/use-recipe-pin-checks';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipePinForm from './RecipePinForm';

afterEach(() => {
  useRecipePinChecks.getState().set([]);
  clearMocks();
});

const PIN: RecipeLayer = {
  id: 'l-9',
  kind: 'pin',
  bone: 'leftFoot',
  settings: {
    weight: 1,
    start: 0.2,
    end: 0.6,
    target: null,
    rotation: null,
    offset: null,
    cycles: null,
    amount: null,
    easeIn: 3,
    easeOut: 3,
    flat: 'toe-and-heel',
  },
};

async function pinned() {
  await openRecipe('slash-into-block');
  useRecipeEditorStore.getState().edit(d => ({ ...d, layers: [...d.layers, PIN] }));
}

const pin = () => useRecipeEditorStore.getState().editing?.draft.layers.find(l => l.id === 'l-9');

describe('the pin settings', () => {
  it('shows the foot, where it stays, keep flat and its eases', async () => {
    await pinned();
    render(<RecipePinForm id="l-9" />);
    expect(screen.getByRole('group', { name: 'Pin, left foot' })).toBeVisible();
    expect(screen.getByText(/^holds \d+ to \d+$/)).toBeVisible();
    expect(screen.getByLabelText('Stays at')).toHaveDisplayValue(/^Where it is at frame \d+$/);
    expect(screen.getByRole('button', { name: 'Toe and heel' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(screen.getByLabelText('Eases in')).toHaveValue('3');
    expect(screen.queryByText('Hand pins take a blank')).toBeNull();
  });

  it('edits the flat and the eases, and drops flat for a hand', async () => {
    await pinned();
    render(<RecipePinForm id="l-9" />);
    await userEvent.click(screen.getByRole('button', { name: 'Heel' }));
    expect(pin()?.settings.flat).toBe('heel');
    await userEvent.clear(screen.getByLabelText('Eases out'));
    await userEvent.type(screen.getByLabelText('Eases out'), '6{Enter}');
    expect(pin()?.settings.easeOut).toBe(6);
    await userEvent.click(screen.getByRole('button', { name: 'L hand' }));
    expect(pin()).toMatchObject({ bone: 'leftHand', settings: { flat: null } });
    expect(screen.queryByText('Keep flat')).toBeNull();
    expect(screen.getByText('Hand pins take a blank')).toBeVisible();
  });

  it('says what the preview found about the spot', async () => {
    await pinned();
    useRecipePinChecks.getState().set([{ layer: 'l-9', hips: 0.31, limb: 0.88, outFrame: null }]);
    render(<RecipePinForm id="l-9" />);
    expect(screen.getByText('The foot can stay on its spot')).toBeVisible();
    expect(screen.getByText(/The hips move 31 cm/)).toBeVisible();
  });
});
