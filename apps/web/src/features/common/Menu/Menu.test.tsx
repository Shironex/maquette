import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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

  it('gives focus back to its trigger when it closes', async () => {
    const onClose = vi.fn();
    const view = (open: boolean) => (
      <div className="relative">
        <button type="button">More for Lamp</button>
        {open ? <Menu label="Card actions" items={items()} onClose={onClose} /> : null}
      </div>
    );
    const { rerender } = render(view(false));
    const trigger = screen.getByRole('button', { name: 'More for Lamp' });
    trigger.focus();
    rerender(view(true));
    expect(screen.getByRole('menuitem', { name: 'Rename' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
    rerender(view(false));
    expect(trigger).toHaveFocus();
  });

  it('moves with the arrow keys and parts its sections with a line', async () => {
    const onSelect = vi.fn();
    render(
      <Menu
        label="Card actions"
        onClose={vi.fn()}
        items={[
          { id: 'send', label: 'Send to 3D', section: 'make', onSelect },
          { id: 'rename', label: 'Rename', section: 'asset', onSelect },
          { id: 'size', label: 'Real size', section: 'asset', onSelect },
        ]}
      />
    );
    expect(screen.getAllByRole('separator')).toHaveLength(1);
    expect(screen.getByRole('menuitem', { name: 'Send to 3D' })).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(screen.getByRole('menuitem', { name: 'Real size' })).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(screen.getByRole('menuitem', { name: 'Send to 3D' })).toHaveFocus();
    await userEvent.keyboard('{End}');
    expect(screen.getByRole('menuitem', { name: 'Real size' })).toHaveFocus();
  });

  it('closes only the submenu on Escape', async () => {
    const onClose = vi.fn();
    render(
      <Menu
        label="Card actions"
        onClose={onClose}
        items={[
          {
            id: 'move',
            label: 'Move to',
            submenu: [{ id: 'refs', label: 'Refs', onSelect: vi.fn() }],
          },
        ]}
      />
    );
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByRole('menuitem', { name: 'Refs' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole('menuitem', { name: 'Move to' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('names a section with a labelled group, and keeps the keys moving through it', async () => {
    render(
      <Menu
        label="Actions"
        onClose={vi.fn()}
        sections={{ make: 'Make' }}
        items={[
          { id: 'a', label: 'Send', section: 'make' },
          { id: 'b', label: 'Refs', section: 'make' },
          { id: 'c', label: 'Remove', section: 'last' },
        ]}
      />
    );
    const group = await screen.findByRole('group', { name: 'Make' });
    expect(
      within(group)
        .getAllByRole('menuitem')
        .map(i => i.textContent)
    ).toEqual(['Send', 'Refs']);
    expect(screen.getByRole('menuitem', { name: 'Remove' }).closest('[role="group"]')).toBeNull();
    await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Send' })).toHaveFocus());
    await userEvent.keyboard('{End}');
    expect(screen.getByRole('menuitem', { name: 'Remove' })).toHaveFocus();
  });

  it('drops the second click of a double-click just after opening, not a later or keyboard one', async () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    try {
      const onSelect = vi.fn();
      render(
        <Menu label="Actions" onClose={vi.fn()} items={[{ id: 'a', label: 'Go', onSelect }]} />
      );
      const row = await screen.findByRole('menuitem', { name: 'Go' });
      fireEvent.click(row, { detail: 2 });
      expect(onSelect).not.toHaveBeenCalled();
      fireEvent.click(row, { detail: 0 });
      expect(onSelect).toHaveBeenCalledTimes(1);
      vi.advanceTimersByTime(300);
      fireEvent.click(row, { detail: 2 });
      expect(onSelect).toHaveBeenCalledTimes(2);
    } finally {
      vi.useRealTimers();
    }
  });
});
