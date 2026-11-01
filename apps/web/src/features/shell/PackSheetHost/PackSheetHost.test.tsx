import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';
import { PackSheetHost } from '.';

vi.mock('@/features/library/PackSheet', () => {
  throw new Error('Failed to fetch the chunk');
});

afterEach(() => {
  vi.restoreAllMocks();
  usePackStore.setState({ open: false, record: null });
  useAppStore.setState({ workspace: null });
});

describe('the pack sheet host', () => {
  it('renders nothing while closed', () => {
    const { container } = render(<PackSheetHost />);
    expect(container).toBeEmptyDOMElement();
  });

  it('says so when the chunk fails, leaves the rest on screen, and closes', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <>
        <button type="button">Approve and send</button>
        <PackSheetHost />
      </>
    );
    act(() => usePackStore.setState({ open: true }));
    const sheet = await screen.findByRole('dialog', { name: 'Export pack could not open' });
    expect(sheet).toHaveTextContent('Nothing was exported');
    expect(screen.getByText('Approve and send')).toBeInTheDocument();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(usePackStore.getState().open).toBe(false));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('closes the failure sheet when another project opens', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const project = (id: string) => ({ workspace: { project: { id } } }) as never;
    act(() => useAppStore.setState(project('one')));
    render(<PackSheetHost />);
    act(() => usePackStore.setState({ open: true }));
    await screen.findByRole('dialog', { name: 'Export pack could not open' });
    act(() => useAppStore.setState(project('two')));
    await waitFor(() => expect(usePackStore.getState().open).toBe(false));
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
