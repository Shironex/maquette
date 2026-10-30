import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { avatarCalls, clearMocks, openAvatar } from '@/test/open-avatar';
import AvatarTabs from './AvatarTabs';

afterEach(() => clearMocks());

describe('the avatar tabs', () => {
  it('build Export, Springs and Preview, switch between them and show the rest as later', async () => {
    await openAvatar();
    render(<AvatarTabs />);
    expect(screen.getByRole('tab', { name: 'Export' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('Expressions')).toHaveAttribute(
      'title',
      expect.stringMatching(/planned/)
    );
    await userEvent.click(screen.getByRole('tab', { name: 'Springs' }));
    expect(useAvatarStore.getState().tab).toBe('springs');
    await userEvent.click(screen.getByRole('tab', { name: 'Preview' }));
    expect(useAvatarStore.getState().tab).toBe('preview');
    expect(screen.getByRole('tab', { name: 'Preview' })).toHaveAttribute('aria-selected', 'true');
  });

  it('lists the versions, marks the one that waits and reloads on a pick', async () => {
    await openAvatar('older');
    render(<AvatarTabs />);
    const picker = screen.getByRole('combobox', { name: 'Model version' });
    expect(picker).toHaveValue('3');
    expect(screen.getByRole('option', { name: 'v2, waits for you' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'v3' })).toBeInTheDocument();
    await userEvent.selectOptions(picker, '2');
    await waitFor(() => expect(useAvatarStore.getState().view?.version).toBe(2));
    expect(avatarCalls.at(-1)).toMatchObject({ cmd: 'avatar_get', args: { version: 2 } });
    expect(useAvatarStore.getState().view?.normalized?.state).toBe('pending');
    expect(screen.getByRole('combobox', { name: 'Model version' })).toHaveValue('2');
  });
});
