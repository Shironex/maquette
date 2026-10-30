import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import RestPosePanel from './RestPosePanel';

afterEach(() => clearMocks());

describe('the rest pose panel', () => {
  it('measures the pose as it is and lets the elbows be kept', async () => {
    await openAvatar();
    render(<RestPosePanel />);
    expect(screen.getByRole('row', { name: /Facing off \+Z 90°/ })).toBeInTheDocument();
    const elbows = screen.getByRole('group', { name: 'Elbows' });
    await userEvent.click(within(elbows).getByRole('button', { name: 'Keep' }));
    expect(useAvatarStore.getState().elbows).toBe('keep');
  });

  it('measures the finger curl and lets the fingers be kept', async () => {
    await openAvatar();
    render(<RestPosePanel />);
    expect(screen.getByRole('row', { name: /Most finger curl 64°/ })).toBeInTheDocument();
    expect(useAvatarStore.getState().fingers).toBe('straighten');
    const fingers = screen.getByRole('group', { name: 'Fingers' });
    await userEvent.click(within(fingers).getByRole('button', { name: 'Keep' }));
    expect(useAvatarStore.getState().fingers).toBe('keep');
  });

  it('asks for a yes on the sheets, and a no writes nothing', async () => {
    await openAvatar('pending');
    render(<RestPosePanel />);
    const ask = screen.getByRole('region', { name: 'Rest pose approval' });
    expect(screen.getByRole('row', { name: /Left arm off level 55° 0°/ })).toBeInTheDocument();
    expect(screen.getByText(/Turned 90°, lifted the left arm 55°/)).toBeInTheDocument();
    await userEvent.click(within(ask).getByRole('button', { name: 'Deny' }));
    expect(await screen.findByText('You turned this pose down')).toBeInTheDocument();
    expect(useAvatarStore.getState().view?.export).toBeNull();
  });

  it('opens the shoulders before and after side by side', async () => {
    await openAvatar('pending');
    render(<RestPosePanel />);
    // The freed arms come first, with sheets of their own.
    const compare = screen.getAllByRole('button', { name: 'Compare the shoulders' });
    expect(compare).toHaveLength(2);
    await userEvent.click(compare[1]!);
    const zoom = screen.getByRole('dialog', { name: 'Rest pose sheets' });
    expect(within(zoom).getAllByRole('img', { name: /the shoulders in four poses/ })).toHaveLength(
      2
    );
  });

  it('opens both hands before and after side by side', async () => {
    await openAvatar('pending');
    render(<RestPosePanel />);
    const compare = screen.getAllByRole('button', { name: 'Compare the hands' });
    await userEvent.click(compare.at(-1)!);
    const zoom = screen.getByRole('dialog', { name: 'Rest pose sheets' });
    expect(within(zoom).getAllByRole('img', { name: /both hands/ })).toHaveLength(2);
  });
});
