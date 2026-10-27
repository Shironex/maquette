import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CompareGroup } from '../compare-options';
import ComparePopover from './ComparePopover';

function groups(onSelect = vi.fn()): CompareGroup[] {
  const option = (id: string, label: string) => ({
    id,
    label,
    icon: 'cube' as const,
    current: false,
    onSelect: () => onSelect(id),
  });
  return [
    { id: 'p', title: 'Props', options: [option('a', 'Bench'), option('b', 'Brazier')] },
    { id: 'c', title: 'Characters', options: [option('c', 'Keeper')] },
  ];
}

describe('the compare popover', () => {
  it('moves through the matches with the arrow keys and picks with Enter', async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(<ComparePopover label="Compare" groups={groups(onSelect)} onClose={onClose} />);
    const search = screen.getByRole('combobox');
    expect(search).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{Enter}');
    expect(onSelect).toHaveBeenCalledWith('b');
    expect(onClose).toHaveBeenCalled();
  });

  it('names the active option on the search box', async () => {
    render(<ComparePopover label="Compare" groups={groups()} onClose={vi.fn()} />);
    const search = screen.getByRole('combobox');
    await userEvent.keyboard('{ArrowDown}');
    const active = screen.getByRole('option', { selected: true });
    expect(search.getAttribute('aria-activedescendant')).toBe(active.id);
  });
});
