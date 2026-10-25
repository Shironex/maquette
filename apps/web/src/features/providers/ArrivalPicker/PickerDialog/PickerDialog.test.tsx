import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PickerDialog } from '.';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({ status: null });
});

describe('the picker dialog', () => {
  it('says so when nothing is left to place, and closes on Escape', async () => {
    await openMockWorkspace('configured');
    await useDownloadsStore.getState().load();
    const onClose = vi.fn();
    render(<PickerDialog onClose={onClose} />);
    const dialog = screen.getByRole('dialog', { name: 'Which asset is this?' });
    expect(dialog).toHaveTextContent('Every new download has found its asset.');
    expect(dialog).toHaveTextContent('New in ~/Downloads');
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });
});
