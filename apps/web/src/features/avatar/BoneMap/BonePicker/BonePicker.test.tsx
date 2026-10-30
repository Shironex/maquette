import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import BonePicker from './BonePicker';

afterEach(() => clearMocks());

describe('the bone picker', () => {
  it('searches the joints, says which bone holds one, and leaves a bone out', async () => {
    await openAvatar('custom');
    const onClose = vi.fn();
    render(<BonePicker bone="leftHand" onClose={onClose} />);
    const search = screen.getByRole('combobox', { name: 'Search the joints for Left hand' });
    expect(search).toHaveFocus();
    expect(screen.getByRole('option', { name: 'Automatic, by name' })).toBeInTheDocument();
    await userEvent.type(search, 'elbow');
    expect(screen.queryByRole('option', { name: 'Leave unmapped' })).not.toBeInTheDocument();
    expect(screen.getByRole('option', { name: /^Left elbow/ })).toHaveTextContent(
      'now left lower arm'
    );
    await userEvent.type(search, 'zzz');
    expect(screen.getByText('No joint matches “elbowzzz”.')).toBeInTheDocument();
    await userEvent.clear(search);
    await userEvent.click(screen.getByRole('option', { name: 'Leave unmapped' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(useAvatarStore.getState().view?.spec.bones).toEqual({ leftHand: null });
  });
});
