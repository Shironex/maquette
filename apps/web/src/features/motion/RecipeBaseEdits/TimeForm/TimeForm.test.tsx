import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import TimeForm from './TimeForm';

afterEach(clearMocks);

const time = () => useRecipeEditorStore.getState().editing!.draft.time;

describe('the recipe time edit form', () => {
  it('sets a retime speed from a preset and says how long the range plays', async () => {
    await openRecipe('slash-into-block');
    render(<TimeForm index={0} />);
    expect(screen.getByText(/frames play in \d+ f · Recipe \d+ f shorter/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '2x' }));
    expect(time()[0]?.speed).toBe(2);
  });

  it('edits a freeze and maps settle to its breathe', async () => {
    await openRecipe('slash-into-block');
    render(<TimeForm index={1} />);
    expect(screen.getByText(/recipe gets 12 f longer/)).toBeInTheDocument();
    await userEvent.clear(screen.getByLabelText('Settle'));
    await userEvent.type(screen.getByLabelText('Settle'), '60{Enter}');
    expect(time()[1]?.breathe).toBe(0.6);
    await userEvent.selectOptions(screen.getByLabelText('Kind'), 'hold-over');
    expect(time()[1]?.kind).toBe('hold-over');
    expect(screen.getByText(/keeps its length/)).toBeInTheDocument();
  });

  it('removes the time edit', async () => {
    await openRecipe('slash-into-block');
    render(<TimeForm index={0} />);
    await userEvent.click(screen.getByRole('button', { name: 'Remove time edit' }));
    expect(time()).toHaveLength(1);
  });
});
