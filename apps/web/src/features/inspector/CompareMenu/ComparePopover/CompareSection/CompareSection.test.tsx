import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { CompareGroup } from '../../compare-options';
import CompareSection from './CompareSection';

const group: CompareGroup = {
  id: 'kind-prop',
  title: 'Props',
  options: [
    {
      id: 'a',
      label: 'Brazier',
      detail: '3 versions',
      icon: 'cube',
      current: false,
      onSelect: vi.fn(),
    },
    { id: 'b', label: 'Bench', icon: 'cube', current: true, onSelect: vi.fn() },
  ],
};

describe('a compare section', () => {
  it('titles the group and shows each option with its count', () => {
    render(<CompareSection group={group} listId="l" active={null} onPick={vi.fn()} />);
    expect(screen.getByRole('group', { name: 'Props' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: /Brazier/ })).toHaveTextContent('3 versions');
  });

  it('picks an option on click, but not the open one', async () => {
    const onPick = vi.fn();
    render(<CompareSection group={group} listId="l" active={group.options[0]!} onPick={onPick} />);
    await userEvent.click(screen.getByRole('option', { name: /Bench/ }));
    expect(onPick).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('option', { name: /Brazier/ }));
    expect(onPick).toHaveBeenCalledWith(group.options[0]);
  });
});
