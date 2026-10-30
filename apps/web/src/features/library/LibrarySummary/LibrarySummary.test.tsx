import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import LibrarySummary from './LibrarySummary';

const base = {
  title: 'All models',
  count: '214 models',
  sort: 'recent' as const,
  onSort: () => {},
  onLookAlikes: () => {},
  onUntagged: () => {},
};

describe('the library summary line', () => {
  it('names what is shown with the pairs and the untagged as buttons', async () => {
    const onLookAlikes = vi.fn();
    const onUntagged = vi.fn();
    render(
      <LibrarySummary
        {...base}
        pairs={3}
        untagged={42}
        onLookAlikes={onLookAlikes}
        onUntagged={onUntagged}
      />
    );
    expect(screen.getByRole('heading', { name: 'All models' })).toBeVisible();
    expect(screen.getByText('214 models')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: '3 look-alike groups' }));
    expect(onLookAlikes).toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: '42 untagged' }));
    expect(onUntagged).toHaveBeenCalled();
  });

  it('leaves the pairs and untagged out when there are none, and sorts from its menu', async () => {
    const onSort = vi.fn();
    render(<LibrarySummary {...base} pairs={0} untagged={0} onSort={onSort} />);
    expect(screen.queryByRole('button', { name: /look-alike/ })).toBeNull();
    expect(screen.queryByRole('button', { name: /untagged/ })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Sort: Newest first' }));
    await userEvent.click(screen.getByRole('menuitem', { name: /Most triangles/ }));
    expect(onSort).toHaveBeenCalledWith('heaviest');
  });
});
