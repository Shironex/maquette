import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ReactFlowProvider } from '@xyflow/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { commands, type InboxSnapshot, type PlanLinks } from '@/ipc/client';
import { useInboxStore } from '@/state/use-inbox-store';
import {
  CHANGE_REREAD_MS,
  RUNNING_REREAD_MS,
  usePlanLinksStore,
} from '@/state/use-plan-links-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, fakeFlowLayout, openPlans } from '@/test/open-plan';
import PlanCanvas from './PlanCanvas';

beforeAll(fakeFlowLayout);
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

const renderCanvas = async (id: string) => {
  await openPlans();
  await usePlansStore.getState().openCanvas(id);
  render(
    <ReactFlowProvider>
      <PlanCanvas />
    </ReactFlowProvider>
  );
  return screen.findByRole('main', { name: `${usePlansStore.getState().open?.plan.name} canvas` });
};

describe('PlanCanvas', () => {
  it('shows the canvas nodes, its file and that it is saved', async () => {
    await renderCanvas('combat-ideas');
    expect(screen.getByRole('heading', { name: 'Combat ideas' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('plans/combat-ideas.plan.json · saved');
    expect(screen.getByRole('group', { name: 'Note: Frost veil' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Picture: Veil gate at night' })).toBeInTheDocument();
  });

  it('adds a note from the bar and from the N key, clear of each other', async () => {
    await renderCanvas('boss-arena');
    expect(screen.getByRole('region', { name: 'This canvas is empty' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Note/ }));
    await userEvent.keyboard('{Escape}');
    (document.activeElement as HTMLElement | null)?.blur();
    await userEvent.keyboard('n');
    const nodes = usePlansStore.getState().nodes;
    expect(nodes).toHaveLength(2);
    expect(nodes[0]!.position).not.toEqual(nodes[1]!.position);
    expect(screen.queryByRole('region', { name: 'This canvas is empty' })).toBeNull();
  });

  it('adds a link through its dialog and a picture through the picker', async () => {
    await renderCanvas('boss-arena');
    await userEvent.click(screen.getByRole('button', { name: /^Link/ }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Address' }), 'example.com/ice');
    await userEvent.click(screen.getByRole('button', { name: 'Add link' }));
    expect(await screen.findByRole('group', { name: /^Link:/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Image/ }));
    await vi.waitFor(() =>
      expect(usePlansStore.getState().nodes.map(n => n.data.node.kind)).toEqual(['link', 'image'])
    );
  });

  it('links an asset from the A key, keeping only its id', async () => {
    await renderCanvas('boss-arena');
    await userEvent.keyboard('a');
    const picker = screen.getByRole('dialog', { name: 'Link assets' });
    const board = within(picker).getByRole('region', { name: 'On the Board' });
    await userEvent.click(within(board).getByRole('checkbox', { name: /^Elsheria/ }));
    await userEvent.click(within(picker).getByRole('button', { name: 'Add' }));
    expect(await screen.findByRole('group', { name: 'Asset: Elsheria' })).toBeInTheDocument();
    const added = usePlansStore.getState().nodes[0]?.data.node;
    expect(added).toMatchObject({ kind: 'asset', asset: 'elsheria', title: '', text: '' });
  });

  it('stays open and asks when the save on the way back fails', async () => {
    await renderCanvas('combat-ideas');
    vi.spyOn(commands, 'planSave')
      .mockRejectedValueOnce({ code: 'io', message: 'The file is locked.' })
      .mockRejectedValueOnce({ code: 'io', message: 'Still locked.' });
    usePlansStore.getState().addNode('note', { x: 900, y: 900 });
    await userEvent.click(screen.getByRole('button', { name: 'Plan, back to all canvases' }));
    const dialog = await screen.findByRole('dialog', { name: 'The canvas was not saved' });
    expect(dialog).toHaveTextContent('The file is locked.');
    expect(screen.getByRole('main', { name: 'Combat ideas canvas' })).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Try again' }));
    const again = await screen.findByRole('dialog', { name: 'The canvas was not saved' });
    await userEvent.click(within(again).getByRole('button', { name: 'Leave without saving' }));
    await vi.waitFor(() => expect(usePlansStore.getState().open).toBeNull());
    expect(usePlansStore.getState().list?.find(p => p.id === 'combat-ideas')?.nodes).toBe(6);
  });

  it('goes back to the list, saving first', async () => {
    await renderCanvas('combat-ideas');
    usePlansStore.getState().addNode('note', { x: 900, y: 900 });
    await userEvent.click(screen.getByRole('button', { name: 'Plan, back to all canvases' }));
    await vi.waitFor(() => expect(usePlansStore.getState().open).toBeNull());
    expect(usePlansStore.getState().list?.find(p => p.id === 'combat-ideas')?.nodes).toBe(7);
  });
});

describe('the live asset links', () => {
  /** A direction whose image is being made, so the canvas reads again every few seconds. */
  const making: PlanLinks = {
    assets: [],
    directions: [
      {
        id: 'frozen-lake-arena',
        state: 'running',
        name: 'Frozen lake arena',
        images: 6,
        picked: 1,
        pending: 2,
        done: 1,
        thumbnail: null,
      },
    ],
  };

  it('reads again while a round is made, on focus and once after changes, and stops on leaving', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const read = vi.spyOn(commands, 'planLinks').mockResolvedValue(making);
    await openPlans();
    await usePlansStore.getState().openCanvas('boss-frost-warden');
    const view = render(
      <ReactFlowProvider>
        <PlanCanvas />
      </ReactFlowProvider>
    );
    await vi.waitFor(() => expect(usePlanLinksStore.getState().directions).not.toEqual({}));
    const calls = () => read.mock.calls.length;
    const settled = calls();

    await vi.advanceTimersByTimeAsync(RUNNING_REREAD_MS);
    expect(calls()).toBe(settled + 1);

    window.dispatchEvent(new Event('focus'));
    expect(calls()).toBe(settled + 2);

    // Inbox events in a burst read once, after they settle.
    const snapshot = { dir: '~/inbox', path: '/inbox', watching: true, items: [] };
    for (let n = 0; n < 3; n++) {
      useInboxStore.setState({ snapshot: { ...snapshot } as unknown as InboxSnapshot });
      await vi.advanceTimersByTimeAsync(CHANGE_REREAD_MS / 5);
    }
    expect(calls()).toBe(settled + 2);
    await vi.advanceTimersByTimeAsync(CHANGE_REREAD_MS);
    expect(calls()).toBe(settled + 3);

    view.unmount();
    expect(usePlanLinksStore.getState().directions).toEqual({});
    window.dispatchEvent(new Event('focus'));
    await vi.advanceTimersByTimeAsync(RUNNING_REREAD_MS * 3);
    expect(calls()).toBe(settled + 3);
  });
});
