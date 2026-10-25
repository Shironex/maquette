import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { KeyRow } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ keys: [] });
});

describe('a key row', () => {
  it('saves a pasted key and then shows only its tail', async () => {
    const { sent } = await openProviders();
    render(<KeyRow service="meshy" />);
    expect(screen.getByText('From meshy.ai, Settings, API.')).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText('Meshy API key'), 'msy_live_9876{Enter}');
    expect(sent('provider_key_save')).toEqual([{ service: 'meshy', key: 'msy_live_9876' }]);
    expect(await screen.findByText(/^\u2022{8}9876$/)).toBeInTheDocument();
    expect(screen.queryByLabelText('Meshy API key')).toBeNull();
  });
});
