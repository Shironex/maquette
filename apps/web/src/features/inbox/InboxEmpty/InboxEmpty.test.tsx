import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import InboxEmpty from './InboxEmpty';

describe('InboxEmpty', () => {
  it('names the inbox folder to drop a model into', () => {
    render(<InboxEmpty dir="~/Maquette/inbox" />);
    expect(screen.getByRole('heading', { name: 'Nothing in the inbox' })).toBeInTheDocument();
    expect(screen.getByText(/Or drop a \.glb into ~\/Maquette\/inbox\./)).toBeInTheDocument();
  });

  it('opens the By hand checklist', async () => {
    render(<InboxEmpty />);
    await userEvent.click(screen.getByRole('button', { name: 'By hand checklist' }));
    expect(useProvidersStore.getState().checklistOpen).toBe(true);
    useProvidersStore.setState({ checklistOpen: false });
  });

  it('falls back to a plain name before the folder is known', () => {
    render(<InboxEmpty />);
    expect(screen.getByText(/into the inbox folder\./)).toBeInTheDocument();
  });
});
