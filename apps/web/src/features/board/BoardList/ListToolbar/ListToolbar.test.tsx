import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_PREFS } from '@/state/board-list';
import ListToolbar from './ListToolbar';

describe('ListToolbar', () => {
  it('picks a stage, searches and counts what is left', async () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <ListToolbar prefs={DEFAULT_PREFS} shown={9} total={9} onChange={onChange} />
    );
    expect(screen.getByText('9 assets')).toBeInTheDocument();
    const filter = screen.getByRole('group', { name: 'Stage filter' });
    await userEvent.click(within(filter).getByRole('button', { name: 'Refs' }));
    expect(onChange).toHaveBeenCalledWith({ stage: 'refs' });
    await userEvent.type(screen.getByRole('searchbox', { name: 'Search assets' }), 'l');
    expect(onChange).toHaveBeenCalledWith({ query: 'l' });
    rerender(<ListToolbar prefs={DEFAULT_PREFS} shown={2} total={9} onChange={onChange} />);
    expect(screen.getByText('2 of 9')).toBeInTheDocument();
  });
});
