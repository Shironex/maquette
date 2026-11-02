import { act, render, screen, within } from '@testing-library/react';
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
import { usePlanLogStore } from '@/state/use-plan-log-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, fakeFlowLayout, openPlans } from '@/test/open-plan';
import PlanCanvas from './PlanCanvas';
import { READABLE_ZOOM, SAFE_INSET, reveal } from './PlanCanvas.hooks';

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
    vi.spyOn(commands, 'planApply')
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

describe('PlanCanvas history', () => {
  const COMBAT = { project: 'frostveil', id: 'combat-ideas' };
  const open = async () => {
    const store = await openPlans();
    await usePlansStore.getState().openCanvas('combat-ideas');
    await usePlanLogStore.getState().load('frostveil', 'combat-ideas');
    render(
      <ReactFlowProvider>
        <PlanCanvas />
      </ReactFlowProvider>
    );
    await screen.findByRole('main', { name: 'Combat ideas canvas' });
    return store;
  };
  /** What the folder watcher sets off after a write from outside. */
  const watcherSaw = async () => {
    await usePlansStore.getState().reload();
    await usePlanLogStore.getState().load('frostveil', 'combat-ideas');
  };
  const text = (id: string) =>
    usePlansStore.getState().nodes.find(n => n.id === id)?.data.node.text;

  it('undoes and redoes the person edits with the buttons and the keys', async () => {
    await open();
    const undo = screen.getByRole('button', { name: 'Undo' });
    expect(undo).toHaveAttribute('aria-disabled', 'true');
    act(() => usePlansStore.getState().updateNode('n-01', { text: 'Edited' }));
    await userEvent.click(undo);
    expect(text('n-01')).toMatch(/^Slow time/);
    await userEvent.keyboard('{Meta>}{Shift>}z{/Shift}{/Meta}');
    expect(text('n-01')).toBe('Edited');
    await userEvent.keyboard('{Control>}z{/Control}');
    expect(text('n-01')).toMatch(/^Slow time/);
    await userEvent.keyboard('{Control>}y{/Control}');
    expect(text('n-01')).toBe('Edited');
    await userEvent.click(screen.getByRole('button', { name: 'Redo' }));
    expect(text('n-01')).toBe('Edited');
  });

  it('shows an agent write with a toast and marks, and undoes its run', async () => {
    const store = await open();
    await new Promise(done => setTimeout(done, 5));
    await act(async () => {
      await store.mock('mock_plan_agent', COMBAT);
      await watcherSaw();
    });
    expect(await screen.findByText('Research agent changed the canvas')).toBeInTheDocument();
    expect(screen.getAllByText('added')).toHaveLength(3);
    expect(screen.getByText('edited')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Changes, 1 new from agents' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Undo run' }));
    await vi.waitFor(() =>
      expect(usePlansStore.getState().nodes.map(n => n.id)).not.toContain('n-a1')
    );
    expect(text('n-06')).toBe('Does the veil read on a phone screen?');
    expect(screen.queryByText('Research agent changed the canvas')).not.toBeInTheDocument();
  });

  it('opens the Changes panel in place of the toast', async () => {
    const store = await open();
    await new Promise(done => setTimeout(done, 5));
    await act(async () => {
      await store.mock('mock_plan_agent', COMBAT);
      await watcherSaw();
    });
    await userEvent.click(screen.getByRole('button', { name: 'Show changes' }));
    const panel = screen.getByRole('complementary', { name: 'Changes' });
    // This run, and the one from yesterday.
    expect(within(panel).getAllByRole('group', { name: 'Research agent, 1 change' })).toHaveLength(
      2
    );
    expect(screen.queryByText('Research agent changed the canvas')).not.toBeInTheDocument();
    await userEvent.click(within(panel).getByRole('button', { name: 'Close Changes' }));
    expect(screen.queryByRole('complementary', { name: 'Changes' })).not.toBeInTheDocument();
  });

  it('asks which side wins when an outside edit meets an unsaved one', async () => {
    const store = await open();
    act(() => usePlansStore.getState().updateNode('n-01', { text: 'Mine' }));
    await act(async () => {
      await store.mock('mock_plan_outside', { ...COMBAT, node: 'n-01', text: 'Theirs' });
      await watcherSaw();
    });
    const dialog = await screen.findByRole('dialog', { name: 'The canvas changed outside' });
    expect(dialog).toHaveTextContent('One node or link changed on both sides.');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Use theirs' }));
    expect(text('n-01')).toBe('Theirs');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('closes the clash question on Escape without choosing, and reopens it from the top bar', async () => {
    const store = await open();
    act(() => usePlansStore.getState().updateNode('n-01', { text: 'Mine' }));
    await act(async () => {
      await store.mock('mock_plan_outside', { ...COMBAT, node: 'n-01', text: 'Theirs' });
      await watcherSaw();
    });
    await screen.findByRole('dialog', { name: 'The canvas changed outside' });
    const apply = vi.spyOn(commands, 'planApply');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(usePlansStore.getState().outside).not.toBeNull();
    expect(text('n-01')).toBe('Mine');
    expect(screen.getByRole('status')).toHaveTextContent('not saved, answer the clash');
    expect(await usePlansStore.getState().save()).toBe(false);
    expect(apply).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole('button', { name: 'Answer clash' }));
    const dialog = await screen.findByRole('dialog', { name: 'The canvas changed outside' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Keep mine' }));
    expect(usePlansStore.getState().outside).toBeNull();
    expect(screen.queryByRole('button', { name: 'Answer clash' })).not.toBeInTheDocument();
  });
});

describe('revealing the changed nodes', () => {
  // A 1000 by 800 canvas: the safe part is 908 by 586 pixels.
  const frame = {
    current: {
      getBoundingClientRect: () => ({ left: 0, top: 0, right: 1000, bottom: 800, width: 1000 }),
    },
  } as unknown as Parameters<typeof reveal>[1];
  const fakeFlow = (
    boxes: Record<string, { x: number; y: number; width: number; height: number }>
  ) => {
    const setCenter = vi.fn();
    const fitView = vi.fn();
    const flow = {
      getNode: (id: string) => boxes[id],
      getNodesBounds: (ids: string[]) => {
        const all = ids.flatMap(id => boxes[id] ?? []);
        const x = Math.min(...all.map(b => b.x));
        const y = Math.min(...all.map(b => b.y));
        return {
          x,
          y,
          width: Math.max(...all.map(b => b.x + b.width)) - x,
          height: Math.max(...all.map(b => b.y + b.height)) - y,
        };
      },
      // Zoom 1, the view at the flow's origin.
      flowToScreenPosition: (p: { x: number; y: number }) => p,
      getZoom: () => 1,
      setCenter,
      fitView,
    } as unknown as Parameters<typeof reveal>[0];
    return { flow, setCenter, fitView };
  };
  const node = (x: number, y: number) => ({ x, y, width: 240, height: 120 });

  it('zooms out as far as the controls stay readable to show them all', () => {
    const { flow, setCenter, fitView } = fakeFlow({ a: node(2000, 100), b: node(2700, 100) });
    reveal(flow, frame, ['a', 'b'], true);
    expect(fitView).not.toHaveBeenCalled();
    const [, , options] = setCenter.mock.calls[0] as [number, number, { zoom: number }];
    expect(options.zoom).toBeCloseTo((1000 - SAFE_INSET.left - SAFE_INSET.right) / 940);
    expect(options.zoom).toBeGreaterThanOrEqual(READABLE_ZOOM);
  });

  it('brings the first one into view when they are too far apart to read', () => {
    const { flow, setCenter } = fakeFlow({ a: node(2000, 100), b: node(6000, 100) });
    reveal(flow, frame, ['a', 'b'], true);
    const [x, , options] = setCenter.mock.calls[0] as [number, number, { zoom: number }];
    expect(options.zoom).toBe(1);
    const shift = (SAFE_INSET.left - SAFE_INSET.right) / 2;
    expect(x).toBe(2000 + 120 - shift);
  });

  it('leaves the view alone when they are in sight', () => {
    const { flow, setCenter } = fakeFlow({ a: node(200, 200) });
    reveal(flow, frame, ['a'], true);
    expect(setCenter).not.toHaveBeenCalled();
  });
});
