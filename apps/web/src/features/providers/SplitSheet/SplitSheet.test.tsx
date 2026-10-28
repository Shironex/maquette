import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { useSplitStore } from '@/state/use-split-store';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import { SplitSheet } from '.';

const open = (asset: string) =>
  useSplitStore.setState({
    split: { project: 'frostveil', asset, version: null, item: null, inBlender: () => {} },
  });

beforeEach(async () => {
  await openMockParts();
  useProvidersStore.setState({ project: 'frostveil', jobs: [] });
});
afterEach(() => {
  useSplitStore.setState({ split: null });
  clearMocks();
});

describe('SplitSheet', () => {
  it('sends a Tripo split after the question, with its estimate', async () => {
    open('rune-chest');
    render(<SplitSheet />);
    expect(await screen.findByText(/v1 from Meshy · one mesh/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Tripo' }));
    expect(screen.getByText('About 40 credits')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: /Complete hidden parts/ }));
    expect(screen.getByText('About 140 credits')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Split with Tripo' }));
    const ask = screen.getByRole('dialog', { name: 'Split Rune chest with Tripo?' });
    expect(ask).toHaveTextContent(
      '1 segment task and 1 completion task, geometry model. About 140 credits. Tripo cannot cancel once it is sent.'
    );
    await userEvent.click(within(ask).getByRole('button', { name: 'Yes, split' }));
    await waitFor(() => expect(useSplitStore.getState().split).toBeNull());
    const job = useProvidersStore.getState().jobs.find(j => j.split);
    expect(job?.provider).toBe('tripo');
    expect(job?.split?.tripo?.complete).toBe('ai-completion');
    expect(job?.estimate).toBe(140);
  });

  it('keeps Meshy off for a model Meshy did not make', async () => {
    open('frost-lantern');
    render(<SplitSheet />);
    expect(await screen.findByText(/v2 from Tripo/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Meshy' })).toHaveAttribute('aria-disabled', 'true');
  });

  it('needs the part names Meshy looks for, and says the texture bakes back', async () => {
    open('rune-chest');
    render(<SplitSheet />);
    await userEvent.click(await screen.findByRole('button', { name: 'Meshy' }));
    expect(screen.getByText(/the texture bakes back from v1 in Blender/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'By parts' }));
    expect(screen.getByText('Name at least one part for Meshy to look for.')).toBeInTheDocument();
    await userEvent.type(
      screen.getByRole('textbox', { name: 'Add a part name' }),
      'lid, body{Enter}'
    );
    expect(screen.getByText('2 of 10')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Split with Meshy' }));
    expect(screen.getByRole('dialog', { name: 'Split Rune chest with Meshy?' })).toHaveTextContent(
      'Meshy Auto Split, by parts, 2 names, then a bake back in Blender. About 10 credits.'
    );
  });
});
