import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useLabStore } from '@/state/use-lab-store';
import { clearMocks, openMockStyle, openMockWorkspace } from '@/test/mock-ipc';
import { StyleBoardDialog } from '.';

afterEach(() => {
  clearMocks();
  useLabStore.setState({ view: null, selected: null, style: null, busy: null });
});

describe('the style board', () => {
  it('edits the line, the palette and the images, and saves a new revision', async () => {
    await openMockStyle();
    await useLabStore.getState().load('frostveil');
    render(<StyleBoardDialog />);
    useLabStore.getState().openStyle();
    const dialog = await screen.findByRole('dialog', { name: 'Style board' });
    expect(within(dialog).getByText(/^Revision 3/)).toBeVisible();
    const save = within(dialog).getByRole('button', { name: 'Save board' });
    expect(save).toHaveAttribute('aria-disabled', 'true');

    const line = within(dialog).getByRole('textbox', { name: 'Style line' });
    await userEvent.clear(line);
    await userEvent.type(line, 'frosted glass, violet rune light');
    await userEvent.type(within(dialog).getByRole('textbox', { name: 'Colour' }), '#c7b8ff{Enter}');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove #5b4a9e' }));
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'Remove elsheria-sheets_02.png' })
    );
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add style image' }));
    await userEvent.click(
      screen.getByRole('button', { name: 'Add Frozen lake arena, Concept 01' })
    );
    await userEvent.click(save);

    await waitFor(() => expect(useLabStore.getState().style).toBeNull());
    const board = useLabStore.getState().view!.style;
    expect(board.revision).toBe(4);
    expect(board.line).toBe('frosted glass, violet rune light');
    expect(board.palette).toEqual(['#1b2440', '#9fc3ff', '#e8f1ff', '#c7b8ff']);
    expect(board.images.map(i => i.source)).toEqual([
      'concepts/veil-gate-kit/veil-gate-kit_01.png',
      'refs/frozen-lake-concept.png',
      'concepts/frozen-lake-arena/frozen-lake-arena_01.png',
    ]);
    expect(board.images[2]!.file).toBe('style/frozen_lake_arena_01.png');
  });

  it('saves on the submit chord and closes on Escape without saving', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('frostveil');
    render(<StyleBoardDialog />);
    useLabStore.getState().openStyle();
    const dialog = await screen.findByRole('dialog', { name: 'Style board' });
    expect(within(dialog).getByRole('button', { name: 'Close' })).toHaveFocus();
    expect(
      within(dialog).getByText('Every new direction and object ref will carry it')
    ).toBeVisible();
    await userEvent.type(
      within(dialog).getByRole('textbox', { name: 'Style line' }),
      'chunky silhouettes{Meta>}{Enter}{/Meta}'
    );
    await waitFor(() => expect(useLabStore.getState().style).toBeNull());
    expect(useLabStore.getState().view!.style).toMatchObject({
      revision: 1,
      line: 'chunky silhouettes',
    });

    useLabStore.getState().openStyle();
    const again = await screen.findByRole('dialog', { name: 'Style board' });
    await userEvent.type(within(again).getByRole('textbox', { name: 'Style line' }), ' and snow');
    await userEvent.keyboard('{Escape}');
    expect(useLabStore.getState().style).toBeNull();
    expect(useLabStore.getState().view!.style.line).toBe('chunky silhouettes');
  });
});
