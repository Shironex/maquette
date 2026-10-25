import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Menu } from '.';

function items(onSelect = vi.fn()) {
  return [
    { id: 'off', label: 'Not now', disabled: true, onSelect },
    { id: 'rename', label: 'Rename', onSelect },
    { id: 'more', label: 'Move to', keepOpen: true, onSelect },
  ];
}

describe('the menu', () => {
  it('focuses the first enabled item and closes after a pick', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(<Menu label="Card actions" items={items(onSelect)} onClose={onClose} />);
    expect(screen.getByRole('menu', { name: 'Card actions' })).toBeVisible();
    const rename = screen.getByRole('menuitem', { name: 'Rename' });
    expect(rename).toHaveFocus();
    await userEvent.click(rename);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('stays open for an item that leads to more choices', async () => {
    const onClose = vi.fn();
    render(<Menu label="Card actions" items={items()} onClose={onClose} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Move to' }));
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes on Escape and on a click outside', async () => {
    const onClose = vi.fn();
    render(
      <>
        <button type="button">Outside</button>
        <Menu label="Card actions" items={items()} onClose={onClose} />
      </>
    );
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: 'Outside' }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
