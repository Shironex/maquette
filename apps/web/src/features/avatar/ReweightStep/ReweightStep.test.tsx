import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAvatarStore } from '@/state/use-avatar-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import ReweightStep from './ReweightStep';

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

  it('stays away for a model with nothing on its arms', async () => {
    await openAvatar(null, 'clay-figure');
    const { container } = render(<ReweightStep />);
    expect(container).toBeEmptyDOMElement();
  });
});
