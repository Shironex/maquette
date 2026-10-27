import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { MenuRow } from '.';

function moveTo(onSelect = vi.fn()) {
  return {
    id: 'move',
    label: 'Move to',
    submenu: [
      { id: 'refs', label: 'Refs', onSelect },
      { id: 'ship', label: 'Ship', onSelect },
    ],
  };
}

describe('a menu row', () => {
  it('picks a plain item and closes the menu', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(<MenuRow item={{ id: 'rename', label: 'Rename', onSelect }} onClose={onClose} />);
    await userEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('opens its submenu on hover and closes it on leaving', async () => {
    render(<MenuRow item={moveTo()} onClose={vi.fn()} />);
    const parent = screen.getByRole('menuitem', { name: 'Move to' });
    expect(parent).toHaveAttribute('aria-haspopup', 'menu');
    expect(parent).toHaveAttribute('aria-expanded', 'false');
    await userEvent.hover(parent);
    expect(screen.getByRole('menu', { name: 'Move to' })).toBeVisible();
    expect(parent).toHaveAttribute('aria-expanded', 'true');
    await userEvent.unhover(parent);
    expect(screen.queryByRole('menu', { name: 'Move to' })).toBeNull();
  });

  it('opens with ArrowRight on its first item and closes with ArrowLeft', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(<MenuRow item={moveTo(onSelect)} onClose={onClose} />);
    const parent = screen.getByRole('menuitem', { name: 'Move to' });
    parent.focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByRole('menuitem', { name: 'Refs' })).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getByRole('menuitem', { name: 'Ship' })).toHaveFocus();
    await userEvent.keyboard('{ArrowLeft}');
    expect(screen.queryByRole('menu', { name: 'Move to' })).toBeNull();
    expect(parent).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    expect(screen.getByRole('menuitem', { name: 'Refs' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(parent).toHaveFocus();
    expect(onClose).not.toHaveBeenCalled();
    await userEvent.keyboard('{ArrowRight}{Enter}');
    expect(onSelect).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });
});
