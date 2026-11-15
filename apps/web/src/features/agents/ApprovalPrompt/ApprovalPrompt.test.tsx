import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { act } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useClearanceStore } from '@/lib/useFloatingClearance';
import { openAgentSession } from '../openAgentSession';
import {
  APPROVAL_RAIL_WIDTH,
  APPROVAL_TRAY_WIDTH,
  useApprovalTrayStore,
} from './ApprovalPrompt.hooks';
import { ApprovalPrompt } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
  useClearanceStore.setState({ bars: {} });
  useApprovalTrayStore.setState({ folded: null });
});

describe('the approval prompt', () => {
  it('floats the waiting approvals over every view, folded to the rail on Settings', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    expect(screen.getAllByRole('alertdialog')).toHaveLength(2);
    // Settings > Agents lists the cards in place, so the tray starts as its rail with the count.
    act(() => useAppStore.setState({ view: 'settings' }));
    expect(screen.queryByRole('alertdialog')).toBeNull();
    const rail = screen.getByRole('complementary', { name: 'Waiting for approval' });
    expect(rail).toHaveTextContent('2');
    await userEvent.click(within(rail).getByRole('button', { name: 'Show the approval cards' }));
    expect(screen.getAllByRole('alertdialog')).toHaveLength(2);
  });

  it('opens one card of several; a folded one names the ask and has no way to answer', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    const [first, second] = screen.getAllByRole('alertdialog');
    expect(within(first!).getByRole('button', { name: /^Deny/ })).toBeVisible();
    expect(second).toHaveAccessibleName('Send 3 assets to Tripo?');
    expect(second).toHaveTextContent('Waiting for you');
    expect(within(second!).queryByRole('button', { name: /^(Approve|Deny)/ })).toBeNull();
    expect(within(second!).queryAllByRole('listitem')).toHaveLength(0);
    const review = within(second!).getByRole('button', { name: /^Review/ });
    expect(review).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(review);
    expect(within(second!).getAllByRole('listitem')).toHaveLength(3);
    expect(within(second!).getByRole('button', { name: 'Approve and send 3' })).toBeVisible();
    expect(review).toHaveAttribute('aria-expanded', 'true');
  });

  it('shows the folder and command with Approve, whenever a preview card is opened', async () => {
    await openAgentSession('many');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    const card = screen.getByRole('alertdialog', {
      name: 'Run the dev command of frostveil-site?',
    });
    expect(within(card).queryByRole('button', { name: 'Approve and run' })).toBeNull();
    expect(card).not.toHaveTextContent('pnpm run dev');
    await userEvent.click(within(card).getByRole('button', { name: /^Review/ }));
    expect(card).toHaveTextContent('/Users/you/code/frostveil-site');
    expect(card).toHaveTextContent('pnpm run dev --port {port}');
    expect(card).toHaveTextContent('astro dev');
    expect(within(card).getByRole('button', { name: 'Approve and run' })).toBeVisible();
  });

  it('docks in a tray beside the view instead of floating over it', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    const tray = screen.getByRole('complementary', { name: 'Waiting for approval' });
    expect(tray).toHaveStyle({ width: `${APPROVAL_TRAY_WIDTH}px` });
    expect(tray).toHaveClass('flex-none');
    expect(tray).not.toHaveClass('fixed');
    expect(within(tray).getAllByRole('alertdialog')).toHaveLength(2);
  });

  it('folds to a rail with the count and opens again, keeping every card waiting', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    await userEvent.click(screen.getByRole('button', { name: 'Fold the approval cards' }));
    const rail = screen.getByRole('complementary', { name: 'Waiting for approval' });
    expect(rail).toHaveStyle({ width: `${APPROVAL_RAIL_WIDTH}px` });
    expect(rail).toHaveTextContent('2');
    expect(screen.queryByRole('alertdialog')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Show the approval cards' }));
    expect(screen.getAllByRole('alertdialog')).toHaveLength(2);
  });

  it('starts folded in a window too narrow for a view beside the tray', async () => {
    const narrow = vi.fn((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }));
    vi.stubGlobal('matchMedia', narrow);
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    expect(screen.getByRole('button', { name: 'Show the approval cards' })).toBeVisible();
    expect(screen.queryByRole('alertdialog')).toBeNull();
    vi.unstubAllGlobals();
  });

  it('stays in its tray when a view registers a bottom bar', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'inbox' }));
    act(() => useClearanceStore.getState().set('rig-doctor-bar', 110));
    render(<ApprovalPrompt />);
    const tray = screen.getByRole('complementary', { name: 'Waiting for approval' });
    expect(tray.style.bottom).toBe('');
    expect(tray.style.maxHeight).toBe('');
  });

  it('renders nothing with no approvals', async () => {
    await openAgentSession(null);
    const { container } = render(<ApprovalPrompt />);
    expect(container).toBeEmptyDOMElement();
  });
});
