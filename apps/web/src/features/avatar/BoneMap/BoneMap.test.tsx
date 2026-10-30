import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { avatarCalls, clearMocks, openAvatar } from '@/test/open-avatar';
import BoneMap from './BoneMap';

afterEach(() => clearMocks());

describe('the bone map', () => {
  it('shows the required bones and the rest on asking', async () => {
    await openAvatar();
    render(<BoneMap />);
    expect(screen.getByText('Required · 15 of 15')).toBeInTheDocument();
    expect(screen.getByText('mixamorig:LeftArm')).toBeInTheDocument();
    expect(screen.queryByText('Left eye')).not.toBeInTheDocument();
    expect(screen.getByText('Optional body · 7 of 10')).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole('button', { name: 'Show' })[0]!);
    expect(screen.getByText('Left eye').closest('li')).toHaveTextContent('missing');
  });

  it('maps a bone by hand from a search, saves it and reads the readiness again', async () => {
    await openAvatar('custom');
    render(<BoneMap />);
    expect(screen.getByText(/2 required bones are missing/)).toBeInTheDocument();
    const row = () => screen.getByText('Left upper leg').closest('li')!;
    expect(row()).toHaveTextContent('missing');
    expect(screen.getByText('Left lower leg').closest('li')).toHaveTextContent(
      'auto · descriptive'
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'Pick the joint for Left upper leg' })
    );
    const search = screen.getByRole('combobox', { name: 'Search the joints for Left upper leg' });
    await userEvent.type(search, 'left th');
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(row()).toHaveTextContent('Left thigh'));
    expect(row()).toHaveTextContent('set by you');
    expect(avatarCalls.findLast(c => c.cmd === 'avatar_save')?.args.spec).toMatchObject({
      bones: { leftUpperLeg: 'Left thigh' },
    });
    expect(useAvatarStore.getState().view?.readiness?.required).toBe(14);

    // Back to automatic: the override is dropped.
    await userEvent.click(
      screen.getByRole('button', { name: 'Pick the joint for Left upper leg' })
    );
    await userEvent.click(screen.getByRole('option', { name: 'Automatic, by name' }));
    await waitFor(() => expect(row()).toHaveTextContent('missing'));
    expect(useAvatarStore.getState().view?.spec.bones).toEqual({});
  });

  it('closes the joint list with Escape', async () => {
    await openAvatar('custom');
    render(<BoneMap />);
    await userEvent.click(screen.getByRole('button', { name: 'Pick the joint for Head' }));
    expect(screen.getByRole('listbox', { name: 'Joints for Head' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('listbox', { name: 'Joints for Head' })).not.toBeInTheDocument();
  });
});
