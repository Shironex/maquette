import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Approval } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { ARM_DELAY_MS } from '../ApprovalCard/ApprovalCard.hooks';
import { openAgentSession } from '../openAgentSession';
import { ApprovalPrompt } from '.';

const waited = (ms: number) => act(() => vi.advanceTimersByTime(ms));

/** The session's approvals on the Board, with `decide` a spy, and fake timers from here on. */
async function session() {
  await openAgentSession('batch');
  const decide = vi.fn(async () => {});
  useAgentsStore.setState({ decide });
  act(() => useAppStore.setState({ view: 'board' }));
  vi.useFakeTimers();
  return decide;
}

const setApprovals = (next: (all: Approval[]) => Approval[]) =>
  act(() => {
    const status = useAgentsStore.getState().status!;
    useAgentsStore.setState({ status: { ...status, approvals: next(status.approvals) } });
  });

const SEND = { name: 'Send 3 assets to Tripo?' };
const approve = (card: HTMLElement) => within(card).getByRole('button', { name: /^Approve/ });

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
  useAgentsStore.setState({ status: null, deciding: null });
});

describe('arming Approve', () => {
  it('ignores a click right after Review opens the card, and is not queued', async () => {
    const decide = await session();
    render(<ApprovalPrompt />);
    waited(ARM_DELAY_MS);
    const card = screen.getByRole('alertdialog', SEND);
    fireEvent.click(within(card).getByRole('button', { name: /^Review/ }));
    fireEvent.click(approve(card));
    expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
    waited(ARM_DELAY_MS - 1);
    fireEvent.click(approve(card));
    waited(ARM_DELAY_MS);
    expect(decide).not.toHaveBeenCalled();
    expect(approve(card)).not.toHaveAttribute('aria-disabled');
    fireEvent.click(approve(card));
    expect(decide).toHaveBeenCalledTimes(1);
  });

  it('holds Approve of a new arrival that opens by itself', async () => {
    const decide = await session();
    const all = useAgentsStore.getState().status!.approvals;
    setApprovals(() => []);
    render(<ApprovalPrompt />);
    setApprovals(() => all.slice(1));
    const card = screen.getByRole('alertdialog', SEND);
    fireEvent.click(approve(card));
    expect(decide).not.toHaveBeenCalled();
    waited(ARM_DELAY_MS);
    fireEvent.click(approve(card));
    expect(decide).toHaveBeenCalledTimes(1);
  });

  it('holds an open card when another is answered, and leaves Deny live', async () => {
    const decide = await session();
    render(<ApprovalPrompt />);
    const card = screen.getByRole('alertdialog', SEND);
    fireEvent.click(within(card).getByRole('button', { name: /^Review/ }));
    waited(ARM_DELAY_MS);
    expect(approve(card)).not.toHaveAttribute('aria-disabled');
    setApprovals(all => all.slice(1));
    fireEvent.click(approve(card));
    expect(decide).not.toHaveBeenCalled();
    fireEvent.click(within(card).getByRole('button', { name: 'Deny' }));
    expect(decide).toHaveBeenCalledWith('approval-2', false);
  });

  it('holds again when a card is hidden and reviewed again, however long after', async () => {
    const decide = await session();
    render(<ApprovalPrompt />);
    const card = screen.getByRole('alertdialog', SEND);
    fireEvent.click(within(card).getByRole('button', { name: /^Review/ }));
    waited(ARM_DELAY_MS);
    fireEvent.click(within(card).getByRole('button', { name: /^Hide/ }));
    waited(2000);
    fireEvent.click(within(card).getByRole('button', { name: /^Review/ }));
    fireEvent.click(approve(card));
    expect(decide).not.toHaveBeenCalled();
    waited(ARM_DELAY_MS);
    fireEvent.click(approve(card));
    expect(decide).toHaveBeenCalledTimes(1);
  });

  it('restarts the hold when a card arrives and leaves inside it', async () => {
    const decide = await session();
    const all = useAgentsStore.getState().status!.approvals;
    setApprovals(() => all.slice(1));
    render(<ApprovalPrompt />);
    waited(ARM_DELAY_MS);
    setApprovals(() => all);
    waited(100);
    setApprovals(() => all.slice(1));
    const card = screen.getByRole('alertdialog', SEND);
    fireEvent.click(approve(card));
    waited(ARM_DELAY_MS - 1);
    fireEvent.click(approve(card));
    expect(decide).not.toHaveBeenCalled();
    waited(1);
    fireEvent.click(approve(card));
    expect(decide).toHaveBeenCalledTimes(1);
  });

  describe('when the cards really move', () => {
    let notify: ResizeObserverCallback = () => {};
    const disconnect = vi.fn();
    const height = (target: Element, h: number) =>
      act(() =>
        notify(
          [{ target, contentRect: { height: h } } as ResizeObserverEntry],
          {} as ResizeObserver
        )
      );

    beforeEach(() => {
      disconnect.mockClear();
      vi.stubGlobal(
        'ResizeObserver',
        class {
          constructor(callback: ResizeObserverCallback) {
            notify = callback;
          }
          observe() {}
          unobserve() {}
          disconnect = disconnect;
        }
      );
    });

    afterEach(() => vi.unstubAllGlobals());

    /** The first card, open and armed. */
    async function armedCard() {
      const decide = await session();
      render(<ApprovalPrompt />);
      waited(ARM_DELAY_MS);
      const card = screen.getAllByRole('alertdialog')[0]!;
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
      return { decide, card };
    }

    it('holds when a card grows after its first measure, as late Ship notes do', async () => {
      const { card } = await armedCard();
      height(card, 200);
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
      height(card, 260);
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
      waited(ARM_DELAY_MS);
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
    });

    it('settles through changes too small to move a button', async () => {
      const { card } = await armedCard();
      height(card, 200);
      for (let i = 0; i < 20; i += 1) {
        height(card, 200 + (i % 2));
        waited(100);
      }
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
    });

    it('holds on a scroll of the column and a resize of the window, not a scroll elsewhere', async () => {
      const { card } = await armedCard();
      fireEvent.scroll(document.createElement('div'));
      const other = document.body.appendChild(document.createElement('div'));
      fireEvent.scroll(other);
      other.remove();
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
      fireEvent.scroll(card.parentElement!);
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
      waited(ARM_DELAY_MS);
      fireEvent(window, new Event('resize'));
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
      waited(ARM_DELAY_MS);
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
    });

    it('lets go of its observer and listeners', async () => {
      await session();
      const { unmount } = render(<ApprovalPrompt />);
      unmount();
      expect(disconnect).toHaveBeenCalled();
      expect(vi.getTimerCount()).toBe(0);
    });
  });

  it('leaves no timer behind when it goes away', async () => {
    await session();
    const { unmount } = render(<ApprovalPrompt />);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
