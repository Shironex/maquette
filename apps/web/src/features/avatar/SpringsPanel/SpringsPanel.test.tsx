import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { avatarCalls, clearMocks, openAvatar } from '@/test/open-avatar';
import SpringsPanel from './SpringsPanel';

afterEach(() => clearMocks());

describe('the springs tab', () => {
  it('waits for the rest pose the springs are found on', async () => {
    await openAvatar();
    render(<SpringsPanel />);
    expect(screen.getByText('The rest pose comes first')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Go to Export' }));
    expect(useAvatarStore.getState().tab).toBe('export');
  });

  it('finds the parts, confirms one with its joints and swing, and chains it for a yes', async () => {
    await openAvatar('written');
    render(<SpringsPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'Find dangling parts' }));
    const list = await screen.findByRole('list', { name: 'Parts that could swing' });
    const strip = within(list).getByRole('listitem', { name: /hair-1, hair from the head/ });
    expect(within(strip).getByText(/62 cm long, 792 vertices/)).toBeInTheDocument();
    expect(
      screen.getByRole('img', { name: 'The parts found, each in its colour sheet' })
    ).toBeVisible();
    // Nothing swings until a person confirms it.
    expect(screen.getByRole('button', { name: 'Make chains' })).toBeDisabled();
    await userEvent.click(within(strip).getByRole('switch', { name: 'Swing hair-1' }));
    await userEvent.selectOptions(
      within(strip).getByRole('combobox', { name: 'How hair-1 swings' }),
      'heavy'
    );
    const spring = useAvatarStore.getState().draft?.springs?.[0];
    expect(spring).toMatchObject({ id: 'hair-1', rootBone: 'head', preset: 'heavy' });

    await userEvent.click(screen.getByRole('button', { name: 'Make chains' }));
    const ask = await screen.findByRole('region', { name: 'Spring chains approval' });
    expect(avatarCalls.some(c => c.cmd === 'avatar_save')).toBe(true);
    expect(screen.getByText(/1 chain \(hair-1\) with 5 joints/)).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'The sway test sheet' })).toBeInTheDocument();

    await userEvent.click(within(ask).getByRole('button', { name: 'Approve and write' }));
    await waitFor(() => expect(useAvatarStore.getState().view?.export?.springs).toBe(1));
    expect(await screen.findByText(/The VRM carries these springs/)).toBeInTheDocument();
  });

  it('answers an agent’s chains, and a no writes the vrm without springs', async () => {
    await openAvatar('chains');
    expect(useAgentsStore.getState().status?.approvals.some(a => a.kind === 'springs')).toBe(true);
    render(<SpringsPanel />);
    const ask = screen.getByRole('region', { name: 'Spring chains approval' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Deny' }));
    expect(await screen.findByText('You turned these chains down')).toBeInTheDocument();
    expect(useAvatarStore.getState().view?.export?.springs).toBe(0);
    expect(useAgentsStore.getState().status?.approvals.some(a => a.kind === 'springs')).toBe(false);
  });

  it('opens the sway test up close', async () => {
    await openAvatar('chains');
    render(<SpringsPanel />);
    await userEvent.click(screen.getByRole('button', { name: 'The sway test sheet' }));
    const zoom = screen.getByRole('dialog', { name: 'Spring sheets' });
    expect(within(zoom).getByRole('img', { name: 'The sway test sheet' })).toBeInTheDocument();
    await userEvent.click(within(zoom).getByRole('button', { name: 'Parts found' }));
    expect(within(zoom).getByRole('img', { name: 'The parts found sheet' })).toBeInTheDocument();
  });
});
