import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import InboxEmpty from './InboxEmpty';

describe('InboxEmpty', () => {
  it('names the inbox folder to drop a model into', () => {
    render(<InboxEmpty dir="~/Maquette/inbox" />);
    expect(screen.getByRole('heading', { name: 'Nothing in the inbox' })).toBeInTheDocument();
    expect(
      screen.getByText(/Drop a \.glb from Tripo into ~\/Maquette\/inbox\./)
    ).toBeInTheDocument();
  });

  it('falls back to a plain name before the folder is known', () => {
    render(<InboxEmpty />);
    expect(screen.getByText(/into the inbox folder\./)).toBeInTheDocument();
  });
});
