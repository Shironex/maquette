import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTriageStore } from '@/state/use-triage-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { InboxClear } from '.';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'clear');
  await Promise.all([
    useInboxStore.getState().load(),
    useTriageStore.getState().loadDiscarded(),
    useProvidersStore.getState().load('frostveil'),
  ]);
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
  useTriageStore.setState({ discarded: [] });
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('the clear inbox', () => {
  it('says the inbox is clear, with the waiting sends and what was discarded', () => {
    render(<InboxClear dir="~/Maquette/inbox" watching />);
    expect(screen.getByRole('heading', { name: 'Inbox clear' })).toBeInTheDocument();
    expect(screen.getByText(/drop a \.glb into ~\/Maquette\/inbox/)).toBeInTheDocument();
    expect(screen.getByText('Watching the folder')).toBeInTheDocument();
    expect(screen.getByText('Guardian statues still waits')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Recently discarded' })).toBeInTheDocument();
  });

  it('says when the folder is not watched', () => {
    render(<InboxClear />);
    expect(screen.getByText('The folder is not watched')).toBeInTheDocument();
    expect(screen.getByText(/into the inbox folder/)).toBeInTheDocument();
  });
});
