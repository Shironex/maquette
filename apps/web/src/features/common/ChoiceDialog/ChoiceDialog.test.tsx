import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ChoiceDialog } from '.';

function renderDialog(loading = false) {
  const onDismiss = vi.fn();
  const onKeep = vi.fn();
  const onDrop = vi.fn();
  render(
    <ChoiceDialog
      title="Drop the draft?"
      onDismiss={onDismiss}
      choices={[
        { label: 'Drop', variant: 'danger', onClick: onDrop },
        { label: 'Keep', variant: 'primary', loading, onClick: onKeep },
      ]}
    >
      <p>The draft has not been saved.</p>
    </ChoiceDialog>
  );
  return { onDismiss, onKeep, onDrop };
}

describe('the choice dialog', () => {
  it('asks the question and starts the focus on the last choice', async () => {
    const { onDrop } = renderDialog();
    const dialog = screen.getByRole('dialog', { name: 'Drop the draft?' });
    expect(dialog).toHaveTextContent('The draft has not been saved.');
    expect(screen.getByRole('button', { name: 'Keep' })).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Drop' }));
    expect(onDrop).toHaveBeenCalledTimes(1);
  });

  it('dismisses on Escape and on a backdrop click', async () => {
    const { onDismiss } = renderDialog();
    await userEvent.keyboard('{Escape}');
    expect(onDismiss).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('dialog').parentElement!);
    expect(onDismiss).toHaveBeenCalledTimes(2);
  });

  it('holds every answer while one is busy', () => {
    const { onDismiss } = renderDialog(true);
    expect(screen.getByRole('button', { name: 'Drop' })).toBeDisabled();
    fireEvent.click(screen.getByRole('dialog').parentElement!);
    expect(onDismiss).not.toHaveBeenCalled();
  });
});
