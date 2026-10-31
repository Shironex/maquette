import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { KeyFields } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ keys: [], keysReading: false });
});

describe('the API keys', () => {
  it('say they wait for the Keychain instead of offering a new key', async () => {
    useProvidersStore.setState({ keys: [], keysReading: true });
    render(<KeyFields />);
    expect(screen.getAllByText('Waiting for the Keychain...')).toHaveLength(2);
    expect(screen.queryByLabelText('Tripo API key')).toBeNull();
    act(() =>
      useProvidersStore.setState({
        keys: [{ service: 'tripo', saved: false, tail: null }],
        keysReading: false,
      })
    );
    expect(screen.queryByText('Waiting for the Keychain...')).toBeNull();
    expect(screen.getByLabelText('Tripo API key')).toBeInTheDocument();
  });

  it('drop a test result once its key is removed or replaced', async () => {
    await openMockWorkspace();
    await useProvidersStore.getState().saveKey('tripo', 'tsk_live_1234');
    render(<KeyFields />);
    await userEvent.click(screen.getAllByRole('button', { name: 'Test key' })[0]!);
    expect(await screen.findByRole('status')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Replace' }));
    expect(screen.queryByRole('status')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Keep the old one' }));

    await userEvent.click(screen.getAllByRole('button', { name: 'Test key' })[0]!);
    const row = (await screen.findByRole('status')).parentElement!;
    await userEvent.click(within(row).getByRole('button', { name: 'Remove' }));
    expect(screen.queryByRole('status')).toBeNull();
  });
});
