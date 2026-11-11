import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import GripDetail from './GripDetail';

afterEach(() => {
  clearMocks();
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

/** The demo library read and the panel on `id`; `refresh` shows it as the list now has it. */
async function openOn(id: string) {
  await openMockLibrary('demo');
  await useGripLibraryStore.getState().load();
  const panel = () => {
    const summary = useGripLibraryStore.getState().list?.find(e => e.id === id);
    return summary ? <GripDetail summary={summary} /> : <p>Gone</p>;
  };
  const view = render(panel());
  return { refresh: () => view.rerender(panel()) };
}

describe('a library grip on show', () => {
  it('lists every revision newest first with where it came from', async () => {
    await openOn('longsword-two-hands');
    const revisions = within(screen.getByRole('region', { name: 'Revisions' })).getAllByRole(
      'button'
    );
    expect(revisions.map(r => r.textContent?.slice(0, 2))).toEqual(['r3', 'r2', 'r1']);
    expect(revisions[0]).toHaveTextContent('Saved from Frostveil / scythe-charge-held, frame 30');
  });

  it("shows the picked revision's hands and bones by group", async () => {
    await openOn('longsword-two-hands');
    const r3 = await screen.findByRole('region', { name: 'What r3 holds' });
    expect(within(r3).getByRole('img', { name: /r3 of Longsword, two hands/ })).toBeVisible();
    expect(r3).toHaveTextContent('Holds a longsword with both hands, the right hand holding.');
    const bones = within(r3).getByRole('list', { name: 'Bones' });
    expect(bones).toHaveTextContent('Right fingers15');
    await userEvent.click(screen.getByRole('button', { name: /^r1/ }));
    expect(await screen.findByRole('region', { name: 'What r1 holds' })).toBeVisible();
  });

  it('asks before removing a revision, then the whole grip', async () => {
    const view = await openOn('longsword-two-hands');
    await userEvent.click(screen.getByRole('button', { name: 'Remove r3' }));
    const dialog = await screen.findByRole('dialog', { name: 'Remove r3?' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove r3' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    view.refresh();
    expect(screen.getByRole('button', { name: 'Remove r2' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Remove grip' }));
    const ask = await screen.findByRole('dialog', { name: 'Remove Longsword, two hands?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Remove grip' }));
    await waitFor(() =>
      expect(useGripLibraryStore.getState().list?.some(g => g.id === 'longsword-two-hands')).toBe(
        false
      )
    );
  });

  it('says why the only revision cannot go', async () => {
    await openOn('dagger-reverse');
    await userEvent.click(screen.getByRole('button', { name: 'Remove r1' }));
    const dialog = await screen.findByRole('dialog', { name: 'Remove r1?' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove r1' }));
    expect(await screen.findByText('Not removed')).toBeVisible();
  });
});
