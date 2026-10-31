import { render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { RoundSummary } from '.';

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
  useInboxStore.setState({ snapshot: null });
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('the round summary', () => {
  beforeEach(async () => {
    await openMockWorkspace('configured', undefined, undefined, 'clear');
    await useProvidersStore.getState().load('frostveil');
  });

  it('adds what came to arrived and names the library and the credits', async () => {
    render(<RoundSummary />);
    const round = await screen.findByLabelText('This round');
    expect(within(round).getByText('Arrived').parentElement).toHaveTextContent('43today');
    expect(within(round).getByText('Accepted').parentElement).toHaveTextContent('407 to library');
    expect(within(round).getByText('Redo sent').parentElement).toHaveTextContent('130 credits');
    expect(within(round).getByText('Still waiting').parentElement).toHaveTextContent('1send');
  });

  it('asks from midnight today, so yesterday does not count', async () => {
    const read = vi.spyOn(commands, 'inboxRound');
    render(<RoundSummary />);
    await screen.findByLabelText('This round');
    const since = new Date(read.mock.calls[0]![0]!);
    expect([since.getHours(), since.getMinutes()]).toEqual([0, 0]);
  });

  it('says nothing about a day with nothing in it', async () => {
    vi.spyOn(commands, 'inboxRound').mockResolvedValue({
      accepted: 0,
      toLibrary: 0,
      discarded: 0,
      redoSent: 0,
      redoCredits: 0,
    });
    useProvidersStore.setState({ jobs: [] });
    const { container } = render(<RoundSummary />);
    await vi.waitFor(() => expect(commands.inboxRound).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });
});
