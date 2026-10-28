import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FilterGroup, FilterList, FilterOption } from '.';

describe('the filter list', () => {
  it('groups options under a heading, each with its count and pressed state', async () => {
    const onClick = vi.fn();
    render(
      <FilterList>
        <FilterGroup title="Packs">
          <FilterOption label="Singles" count={4} on onClick={vi.fn()} />
          <FilterOption label="Pro Magic Pack" count={12} on={false} onClick={onClick} />
        </FilterGroup>
      </FilterList>
    );
    const group = within(screen.getByRole('region', { name: 'Filters' })).getByRole('group', {
      name: 'Packs',
    });
    expect(within(group).getByRole('button', { name: /Singles/ })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(within(group).getByRole('button', { name: /Pro Magic Pack/ }));
    expect(onClick).toHaveBeenCalled();
    expect(group).toHaveTextContent('12');
  });
});
