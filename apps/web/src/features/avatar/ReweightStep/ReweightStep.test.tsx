import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { ReweightBridge } from '@/ipc/client';
import { useAvatarStore } from '@/state/use-avatar-store';
import { avatarCalls, clearMocks, openAvatar } from '@/test/open-avatar';
import ReweightStep from './ReweightStep';
import { reweightSummary } from './ReweightStep.hooks';

afterEach(() => clearMocks());

describe('the step that frees cloth and hair from the arms', () => {
  it('says what a build will do while cloth hangs on the arms', async () => {
    await openAvatar();
    render(<ReweightStep />);
    expect(
      screen.getByRole('heading', { name: 'Free cloth and hair from the arms' })
    ).toBeInTheDocument();
    expect(screen.getByText(/212 vertices of cloth or hair weighted to the arms/)).toBeVisible();
  });

  it('asks for a yes on its own sheets, and a no lifts the arms as they are', async () => {
    await openAvatar('reweight');
    render(<ReweightStep />);
    const ask = screen.getByRole('region', { name: 'Cloth and hair approval' });
    expect(screen.getByText(/Moved 2,314 vertices of cloth and hair to the trunk/)).toBeVisible();
    expect(screen.getByRole('img', { name: 'The after sheet' })).toBeInTheDocument();
    await userEvent.click(within(ask).getByRole('button', { name: 'Deny' }));
    expect(await screen.findByText('You turned the new weights down')).toBeInTheDocument();
    expect(useAvatarStore.getState().view?.reweighted?.state).toBe('denied');
  });

  it('shows the shoulders of its three poses side by side', async () => {
    await openAvatar('reweight');
    render(<ReweightStep />);
    await userEvent.click(screen.getByRole('button', { name: 'Compare the shoulders' }));
    const zoom = screen.getByRole('dialog', { name: 'Freed arms sheets' });
    expect(within(zoom).getAllByRole('img', { name: /the shoulders in three poses/ })).toHaveLength(
      2
    );
  });

  it('hides the faces fused to the body by default, and keeping them reweights again', async () => {
    await openAvatar('reweight');
    render(<ReweightStep />);
    expect(screen.getByText(/17 faces joining the right arm to the body are hidden/)).toBeVisible();
    const choice = screen.getByRole('group', { name: 'Faces fused to the body' });
    expect(within(choice).getByRole('button', { name: 'Hide' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(within(choice).getByRole('button', { name: 'Keep' }));
    expect(await screen.findByText(/to the body still stretch when lifted/)).toBeVisible();
    const saved = avatarCalls.find(c => c.cmd === 'avatar_save');
    expect((saved?.args.spec as { bridges?: string }).bridges).toBe('keep');
    expect(avatarCalls.some(c => c.cmd === 'avatar_build')).toBe(true);
    const r = useAvatarStore.getState().view?.reweighted;
    expect(r?.bridges).toBe('keep');
    expect(r?.state).toBe('pending');
    expect(r?.report.hiddenFaces).toBe(0);
  });

  it('stays away for a model with nothing on its arms', async () => {
    await openAvatar(null, 'clay-figure');
    const { container } = render(<ReweightStep />);
    expect(container).toBeEmptyDOMElement();
  });

  it('names the faces fused between an arm and the body that still stretch', () => {
    const base = { moved: 10, keptAsSleeve: 4, fromBones: { RightArm: 10 } };
    expect(reweightSummary(base)).toBe(
      'Moved 10 vertices of cloth and hair to the trunk, most from RightArm; kept 4 on the arms as sleeve.'
    );
    const bridge = (side: string, faces: number): ReweightBridge => ({
      side,
      faces,
      at: [0, 1, 0],
      length: 0.4,
    });
    expect(
      reweightSummary({ ...base, bridges: [bridge('right', 35), bridge('right', 17)] })
    ).toMatch(/sleeve\. 52 faces joining the right arm to the body still stretch when lifted\.$/);
    expect(reweightSummary({ ...base, bridges: [bridge('left', 1)] })).toMatch(
      /1 face joining the left arm to the body still stretches when lifted\.$/
    );
    expect(reweightSummary({ ...base, bridges: [bridge('right', 35)], hiddenFaces: 35 })).toMatch(
      /35 faces joining the right arm to the body are hidden, as they would stretch/
    );
    expect(reweightSummary({ ...base, bridges: [bridge('right', 35)], hiddenFaces: 20 })).toMatch(
      /still stretch when lifted\. 20 of them hidden\.$/
    );
  });
});
