import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Arrival, ToolReport, WatchStatus } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { BlenderBanner } from '.';

const tools = (status: ToolReport['status'], checking = false) =>
  useAppStore.setState({
    tools: { checking, reports: [{ id: 'blender', status } as ToolReport] } as never,
  });
const hold = (n: number) =>
  useDownloadsStore.setState({
    status: {
      arrivals: Array.from(
        { length: n },
        (_, i) => ({ id: `a${i}`, state: 'unmatched', picture: 'held' }) as Arrival
      ),
    } as WatchStatus,
  });

afterEach(() => {
  useDownloadsStore.setState({ status: null });
  vi.restoreAllMocks();
});

describe('the Blender missing banner', () => {
  it('says picture match is off and how many downloads wait, and opens Settings', async () => {
    const setView = vi.fn();
    useAppStore.setState({ setView });
    tools('missing');
    hold(2);
    render(<BlenderBanner />);
    expect(screen.getByRole('status')).toHaveTextContent('Picture match is off');
    expect(screen.getByRole('status')).toHaveTextContent('2 waiting downloads are on hold');
    await userEvent.click(screen.getByRole('button', { name: 'Open Settings' }));
    expect(setView).toHaveBeenCalledWith('settings');
  });

  it('stays away while Blender is found or still being looked for', () => {
    tools('found');
    const { container, rerender } = render(<BlenderBanner />);
    expect(container).toBeEmptyDOMElement();
    tools('missing', true);
    rerender(<BlenderBanner />);
    expect(container).toBeEmptyDOMElement();
  });
});
