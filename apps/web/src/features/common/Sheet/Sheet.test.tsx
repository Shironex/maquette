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

  it('keeps Tab inside and gives focus back to its opener on close', async () => {
    const onClose = vi.fn();
    const view = (open: boolean) => (
      <>
        <button type="button">Send to 3D</button>
        {open ? (
          <Sheet id="t" icon="cube" title="Send Lamp to 3D" onClose={onClose}>
            <button type="button">Send</button>
          </Sheet>
        ) : null}
      </>
    );
    const { rerender } = render(view(false));
    const opener = screen.getByRole('button', { name: 'Send to 3D' });
    opener.focus();
    rerender(view(true));
    const sheet = screen.getByRole('dialog', { name: 'Send Lamp to 3D' });
    expect(within(sheet).getByRole('button', { name: 'Close' })).toHaveFocus();
    await userEvent.tab();
    expect(within(sheet).getByRole('button', { name: 'Send' })).toHaveFocus();
    await userEvent.tab();
    expect(within(sheet).getByRole('button', { name: 'Close' })).toHaveFocus();
    await userEvent.tab({ shift: true });
    expect(within(sheet).getByRole('button', { name: 'Send' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
    rerender(view(false));
    expect(opener).toHaveFocus();
  });
});
