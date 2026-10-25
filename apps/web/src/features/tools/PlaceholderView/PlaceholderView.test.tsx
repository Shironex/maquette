import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { PlaceholderView } from '.';

afterEach(cleanup);

describe('the placeholder view', () => {
  it('titles the view and explains what arrives later', () => {
    render(<PlaceholderView view="inbox" />);
    expect(screen.getByRole('heading', { name: 'Inbox' })).toBeVisible();
    expect(screen.getByText('Nothing in the inbox')).toBeVisible();
  });

  it('shows only the title for a view without copy', () => {
    render(<PlaceholderView view="board" />);
    expect(screen.getByRole('heading', { name: 'Board' })).toBeVisible();
    expect(screen.queryByText(/arrives in a later version/)).toBeNull();
  });
});
