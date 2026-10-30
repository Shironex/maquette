import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import AvatarView from './AvatarView';

afterEach(() => clearMocks());

describe('the Avatar view', () => {
  it('holds the build until the meta names an author', async () => {
    await openAvatar();
    render(<AvatarView />);
    expect(await screen.findByRole('heading', { name: 'Elsheria' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Build VRM' })).toBeDisabled();
    expect(screen.getByRole('region', { name: 'Characters' })).toHaveTextContent('Elsheria');
  });

  it('builds, waits for each yes on the sheets, then shows the file', async () => {
    await openAvatar();
    render(<AvatarView />);
    await userEvent.type(await screen.findByLabelText('Authors'), 'Ana');
    await userEvent.click(screen.getByRole('button', { name: 'Build VRM' }));
    const weights = await screen.findByRole('region', { name: 'Cloth and hair approval' });
    await userEvent.click(within(weights).getByRole('button', { name: 'Approve and go on' }));
    const ask = await screen.findByRole('region', { name: 'Rest pose approval' });
    expect(screen.getAllByRole('img', { name: 'The after sheet' })).toHaveLength(2);
    await userEvent.click(within(ask).getByRole('button', { name: 'Approve and write' }));
    const last = await screen.findByRole('region', { name: 'Last export' });
    expect(last).toHaveTextContent('Last export');
    expect(screen.getByRole('button', { name: 'Build again' })).toBeEnabled();
  });

  it('says so when the project has no characters', async () => {
    await openAvatar();
    const ws = useAppStore.getState().workspace!;
    const assets = (ws.project!.assets ?? []).filter(a => a.kind !== 'character');
    useAppStore.setState({ workspace: { ...ws, project: { ...ws.project!, assets } } });
    render(<AvatarView />);
    await waitFor(() => expect(screen.getByText('No characters yet')).toBeInTheDocument());
  });
});
