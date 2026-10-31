import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type SceneLayout } from '@/ipc/client';
import { usePageStore } from '@/state/use-page-store';
import { PageHotspots } from '.';

const layout = {
  models: { cabinet: { position: [0, 0, 0], rotation: [0, 0, 0], scale: 1 } },
  stops: [{ id: 'a', name: 'A', position: [0, 0, 5], target: [0, 0, 0], fov: 40, models: null }],
  states: {
    cabinet: {
      states: [{ id: 'on', name: 'Breakers on', parts: {} }],
      transitions: [],
      triggers: [{ kind: 'click', to: 'on', stop: null, part: null, start: null, end: null }],
    },
  },
} as unknown as SceneLayout;

afterEach(() => {
  vi.restoreAllMocks();
  usePageStore.setState(usePageStore.getInitialState(), true);
});

describe('the page hotspots', () => {
  it('pins a click trigger', async () => {
    vi.spyOn(commands, 'layoutGet').mockResolvedValue({ layout } as never);
    usePageStore.setState({ project: 'demo' });
    render(<PageHotspots />);
    expect(await screen.findByText('Click · Breakers on')).toBeVisible();
  });

  it('says so when no prop has one', async () => {
    const empty = { ...layout, states: {} } as SceneLayout;
    vi.spyOn(commands, 'layoutGet').mockResolvedValue({ layout: empty } as never);
    usePageStore.setState({ project: 'demo' });
    render(<PageHotspots />);
    expect(await screen.findByText(/^No placed prop has a click or hover trigger/)).toBeVisible();
  });
});
