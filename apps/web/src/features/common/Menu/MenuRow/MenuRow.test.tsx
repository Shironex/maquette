import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { MenuRow } from '.';
import { SUBMENU_WIDTH, placeSubmenu } from './MenuRow.hooks';

const inMenu = (row: ReactNode) => <div role="menu">{row}</div>;

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

  it('shows a danger item in the danger colour', () => {
    render(
      <MenuRow
        item={{ id: 'remove', label: 'Remove', icon: 'trash', tone: 'danger', onSelect: vi.fn() }}
        onClose={vi.fn()}
      />
    );
    expect(screen.getByRole('menuitem', { name: 'Remove' })).toHaveClass('text-danger');
    expect(screen.queryByRole('menuitem', { name: 'Rename' })).toBeNull();
  });

  it('opens its submenu on hover and closes it on leaving', async () => {
    render(inMenu(<MenuRow item={moveTo()} onClose={vi.fn()} />));
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
    render(inMenu(<MenuRow item={moveTo(onSelect)} onClose={onClose} />));
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

  it('places its submenu beside the menu, or to the left at the window edge', () => {
    const rect = (left: number, top: number, width: number) => new DOMRect(left, top, width, 30);
    const menu = rect(100, 100, 200);
    const item = rect(105, 400, 190);
    expect(placeSubmenu(menu, item, 5)).toMatchObject({ left: 295, padLeft: 7, top: 395 });
    const edge = rect(window.innerWidth - 200, 100, 200);
    const atEdge = rect(window.innerWidth - 195, 400, 190);
    const left = placeSubmenu(edge, atEdge, 5);
    expect(left.left + left.padRight + SUBMENU_WIDTH).toBe(atEdge.left);
    expect(placeSubmenu(menu, rect(105, window.innerHeight - 20, 190), 5).top).toBeLessThan(
      window.innerHeight - 160
    );
  });
});
