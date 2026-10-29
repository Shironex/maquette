import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { Arrival } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { LookCard } from '.';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({ status: null, chooserFor: null, previews: {}, busy: [] });
});

/** The Tripo batch export whose name is close to the Lantern post send. */
async function lookAtLantern(): Promise<Arrival> {
  await openMockWorkspace('configured', undefined, 'batch', undefined, 'look');
  let found: Arrival | undefined;
  await waitFor(async () => {
    await useDownloadsStore.getState().load();
    found = useDownloadsStore.getState().status?.arrivals.find(a => a.state === 'review');
    expect(found).toBeDefined();
  });
  render(
    <ul>
      <LookCard arrival={found!} now={found!.atMs ?? 0} />
    </ul>
  );
  return found!;
}

describe('a download that needs a look', () => {
  it('shows the front render next to the front ref, the file, the fit and why', async () => {
    await lookAtLantern();
    expect(screen.getByText('Is this Lantern post?')).toBeInTheDocument();
    expect(screen.getByText('lantern post 3d model.glb')).toBeInTheDocument();
    expect(screen.getByText('Looks like Lantern post (88%)')).toBeInTheDocument();
    expect(screen.getByText('88% alike')).toBeInTheDocument();
    // The model's headless render was asked for; mock mode serves no files in tests.
    const arrival = useDownloadsStore.getState().status?.arrivals.find(a => a.state === 'review');
    await waitFor(() =>
      expect(useDownloadsStore.getState().previews[arrival!.id]?.thumbnail).toMatch(
        /crystal-lantern\.webp$/
      )
    );
    expect(screen.getByText('Download')).toBeInTheDocument();
    expect(screen.getByText('Front ref')).toBeInTheDocument();
    // Its front render, matched against the refs, is the picture shown.
    expect(arrival?.render).toMatch(/crystal-lantern\.webp$/);
    expect(arrival?.picture).toBe('ready');
  });

  it('sends it to the Inbox for the asset it looks like, for that send', async () => {
    const arrival = await lookAtLantern();
    await userEvent.click(screen.getByRole('button', { name: 'Attach to Lantern post' }));
    await waitFor(() =>
      expect(
        useDownloadsStore.getState().status?.arrivals.find(a => a.id === arrival.id)?.state
      ).toBe('attached')
    );
    expect(
      useDownloadsStore.getState().status?.arrivals.find(a => a.id === arrival.id)?.inboxFile
    ).toBe(`${arrival.job}.glb`);
    const asset = useAppStore
      .getState()
      .workspace?.project?.assets?.find(a => a.id === arrival.asset);
    expect(asset?.versions).toHaveLength(0);
    await useDownloadsStore.getState().load();
    expect(useDownloadsStore.getState().status?.pending.some(p => p.job === arrival.job)).toBe(
      false
    );
  });

  it('picks another asset, or leaves it and does not ask again', async () => {
    const arrival = await lookAtLantern();
    await userEvent.click(screen.getByRole('button', { name: 'Pick another asset' }));
    expect(useDownloadsStore.getState().chooserFor).toBe(arrival.id);
    await userEvent.click(screen.getByRole('button', { name: 'Not this one' }));
    await waitFor(() =>
      expect(
        useDownloadsStore.getState().status?.arrivals.find(a => a.id === arrival.id)?.state
      ).toBe('dismissed')
    );
    expect(useDownloadsStore.getState().chooserFor).toBeNull();
  });

  it('ranks the other waiting sends after it, and attaches to one of them instead', async () => {
    const arrival = await lookAtLantern();
    const others = (arrival.suggestions ?? []).filter(s => s.job !== arrival.job);
    expect(others.length).toBeGreaterThan(0);
    expect(screen.getByText('Also waiting:')).toBeInTheDocument();
    const other = others[0]!;
    await userEvent.click(
      screen.getByRole('button', { name: new RegExp(`^${other.assetName} · `) })
    );
    await waitFor(() =>
      expect(
        useDownloadsStore.getState().status?.arrivals.find(a => a.id === arrival.id)?.asset
      ).toBe(other.asset)
    );
  });
});
