import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DeviceVerdict } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

async function open() {
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    'running'
  );
  await usePageStore.getState().open('moon-island');
  await vi.waitFor(() => expect(usePageStore.getState().inspect).not.toBeNull());
  act(() => usePageStore.getState().setDevice('phone'));
}

describe('the device verdict', () => {
  it('lists the phone budget and what changed since the ship', async () => {
    await open();
    render(<DeviceVerdict />);
    expect(screen.getByRole('heading', { name: 'Phone verdict' })).toBeVisible();
    expect(screen.getByText('33 ms, 30 fps')).toBeVisible();
    expect(screen.getByText('Character is new')).toBeVisible();
  });

  it('says when the page runs over the budget', async () => {
    await open();
    render(<DeviceVerdict />);
    act(() =>
      usePageStore.getState().onMessage('ship', {
        source: 'site',
        type: 'stats',
        frameMs: 21.4,
        p95Ms: 38.2,
        drawCalls: 71,
      })
    );
    expect(screen.getByText('This run is over the phone budget')).toBeVisible();
    expect(screen.getByText(/Frame time, p95 38\.2 ms, the phone allows 33 ms/)).toBeVisible();
  });

  it('opens the Inspector', async () => {
    await open();
    render(<DeviceVerdict />);
    await userEvent.click(screen.getByRole('button', { name: 'Open Inspector' }));
    expect(useAppStore.getState().view).toBe('inspector');
  });
});
