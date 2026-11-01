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
    let frames: FrameRequestCallback[] = [];
    let rect = { top: 100, left: 50, width: 400, height: 200 };

    /** Runs one animation frame of the page. */
    const frame = () =>
      act(() => {
        const due = frames;
        frames = [];
        for (const callback of due) callback(0);
      });

    beforeEach(() => {
      rect = { top: 100, left: 50, width: 400, height: 200 };
      vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(
        () => rect as DOMRect
      );
    });

    afterEach(() => {
      vi.unstubAllGlobals();
      vi.restoreAllMocks();
    });

    /** The session on fake timers, with animation frames run by hand. */
    async function paced() {
      await session();
      frames = [];
      vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) =>
        frames.push(callback)
      );
      vi.stubGlobal('cancelAnimationFrame', (id: number) => {
        frames.splice(id - 1, 1);
      });
    }

    /** The first card, open and armed. */
    async function armedCard() {
      await paced();
      render(<ApprovalPrompt />);
      waited(ARM_DELAY_MS);
      const card = screen.getAllByRole('alertdialog')[0]!;
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
      return card;
    }

    it('holds when content above moves the column, and comes back after', async () => {
      const card = await armedCard();
      rect = { ...rect, top: rect.top + 40 };
      frame();
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
      waited(ARM_DELAY_MS - 1);
      frame();
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
      waited(1);
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
    });

    it('stays armed on a column that stays still', async () => {
      const card = await armedCard();
      for (let i = 0; i < 50; i += 1) frame();
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
    });

    it('holds when a card grows, as late Ship notes do', async () => {
      const card = await armedCard();
      rect = { ...rect, height: rect.height + 60 };
      frame();
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
    });

    it('holds when the column slides sideways, as on a resized window', async () => {
      const card = await armedCard();
      rect = { ...rect, left: rect.left - 80 };
      frame();
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
    });

    it('holds when only the width changes, as Approve sits at the right edge', async () => {
      const card = await armedCard();
      rect = { ...rect, width: rect.width - 60 };
      frame();
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
    });

    it('settles through jitter too small to move a button', async () => {
      const card = await armedCard();
      for (let i = 0; i < 20; i += 1) {
        rect = { ...rect, height: 200 + (i % 2) };
        frame();
        waited(100);
      }
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
    });

    it('adds small steps up from the last restart', async () => {
      const card = await armedCard();
      rect = { ...rect, height: rect.height + 1 };
      frame();
      expect(approve(card)).not.toHaveAttribute('aria-disabled');
      rect = { ...rect, height: rect.height + 1 };
      frame();
      expect(approve(card)).toHaveAttribute('aria-disabled', 'true');
    });

    it('looks only while a card is open, and stops on unmount', async () => {
      await paced();
      const { unmount } = render(<ApprovalPrompt />);
      expect(frames).toHaveLength(1);
      const card = screen.getAllByRole('alertdialog')[0]!;
      fireEvent.click(within(card).getByRole('button', { name: /^Hide/ }));
      expect(frames).toHaveLength(0);
      fireEvent.click(within(card).getByRole('button', { name: /^Review/ }));
      expect(frames).toHaveLength(1);
      unmount();
      expect(frames).toHaveLength(0);
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
