import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Arrival, WatchStatus } from '@/ipc/client';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { DownloadsOnHold } from '.';

const arrival = (over: Partial<Arrival>) =>
  ({
    id: 'a',
    name: 'a.glb',
    state: 'unmatched',
    picture: 'held',
    suggestions: [],
    ...over,
  }) as Arrival;
const show = (arrivals: Arrival[]) =>
  useDownloadsStore.setState({ status: { arrivals } as WatchStatus });

afterEach(() => useDownloadsStore.setState({ status: null, pickerOpen: false }));

describe('the downloads on hold', () => {
  it('says what each has to go on and opens the picker to match by hand', async () => {
    const openPicker = vi.fn();
    useDownloadsStore.setState({ openPicker });
    show([
      arrival({
        id: 'w',
        name: 'festive wreath 3d model.glb',
        suggestions: [{ name: 0.6, assetName: 'Wreath stand' } as never],
      }),
      arrival({ id: 's', name: 'snowman 3d model.glb' }),
      arrival({ id: 'done', name: 'placed.glb', state: 'imported' }),
      arrival({ id: 'q', name: 'queued.glb', picture: 'queued' }),
    ]);
    render(<DownloadsOnHold />);
    expect(screen.getByRole('heading', { name: 'Downloads on hold · 2' })).toBeInTheDocument();
    expect(screen.getByText('Waits for a picture. Name is close to Wreath stand.')).toBeVisible();
    expect(screen.getByText('Waits for a picture. No name match.')).toBeVisible();
    expect(screen.queryByText('placed.glb')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Match these by hand' }));
    expect(openPicker).toHaveBeenCalled();
  });

  it('shows nothing when no download waits for Blender', () => {
    show([arrival({ picture: 'ready' })]);
    const { container } = render(<DownloadsOnHold />);
    expect(container).toBeEmptyDOMElement();
  });
});
