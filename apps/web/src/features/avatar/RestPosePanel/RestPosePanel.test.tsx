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
    await userEvent.click(screen.getByRole('button', { name: 'Keep' }));
    expect(useAvatarStore.getState().elbows).toBe('keep');
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
    await userEvent.click(screen.getByRole('button', { name: 'Compare the shoulders' }));
    const zoom = screen.getByRole('dialog', { name: 'Rest pose sheets' });
    expect(within(zoom).getAllByRole('img', { name: /the shoulders in four poses/ })).toHaveLength(
      2
    );
  });
});
