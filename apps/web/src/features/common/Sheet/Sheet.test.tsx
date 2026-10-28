import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Sheet } from '.';

function open(dirty: boolean) {
  const onClose = vi.fn();
  render(
    <Sheet
      id="test"
      icon="cube"
      title="Send Lamp to 3D"
      meta="prop"
      onClose={onClose}
      dirty={dirty}
    >
      <p>Body</p>
    </Sheet>
  );
  return onClose;
}

describe('the sheet', () => {
  it('is a labelled dialog that closes from its button', async () => {
    const onClose = open(false);
    const sheet = screen.getByRole('dialog', { name: 'Send Lamp to 3D' });
    expect(within(sheet).getByText('prop')).toBeInTheDocument();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Close' }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('asks before closing with unsent changes', async () => {
    const onClose = open(true);
    await userEvent.keyboard('{Escape}');
    const ask = screen.getByRole('dialog', { name: 'Discard changes?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Discard' }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
