import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { InboxList } from '.';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null, checked: [] });
  useTriageStore.setState({ filter: 'all', discarded: [], working: false });
  useAppStore.setState({ notice: null });
});

describe('the inbox list column', () => {
  it('filters the models by what they need', async () => {
    render(<InboxList shownId={null} />);
    const list = screen.getByRole('region', { name: 'Models in the inbox' });
    expect(list).toHaveTextContent('12 models waiting');
    await userEvent.click(screen.getByRole('button', { name: 'Replaced 2' }));
    expect(list).toHaveTextContent('2 of 12 models waiting');
    expect(within(list).getAllByRole('listitem')).toHaveLength(2);
  });

  it('accepts the ticked models from the selection bar', async () => {
    render(<InboxList shownId={null} />);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Frozen angel' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Guardian statues' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected models' });
    expect(bar).toHaveTextContent('2 selected · all checks pass');
    await userEvent.click(within(bar).getByRole('button', { name: 'Accept 2' }));
    await vi.waitFor(() => expect(useAppStore.getState().notice?.title).toBe('2 models accepted'));
    expect(useInboxStore.getState().snapshot!.items).toHaveLength(10);
  });

  it('asks before discarding the ticked models', async () => {
    render(<InboxList shownId={null} />);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Veil platform scan' }));
    await userEvent.click(screen.getByRole('button', { name: 'Discard 1' }));
    const dialog = screen.getByRole('dialog', { name: 'Discard 1 model?' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Discard 1' }));
    await vi.waitFor(() =>
      expect(useInboxStore.getState().snapshot!.items.map(i => i.id)).not.toContain(
        'veil_platform_scan.glb'
      )
    );
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
