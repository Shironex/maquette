import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { act } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useClearanceStore } from '@/lib/useFloatingClearance';
import { openAgentSession } from '../openAgentSession';
import { APPROVAL_TRAY_WIDTH } from './ApprovalPrompt.hooks';
import { ApprovalPrompt } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
  useClearanceStore.setState({ bars: {} });
});

describe('the approval prompt', () => {
  it('floats the waiting approvals over every view but Settings', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    render(<ApprovalPrompt />);
    expect(screen.getAllByRole('alertdialog')).toHaveLength(2);
    act(() => useAppStore.setState({ view: 'settings' }));
    expect(screen.queryByRole('alertdialog')).toBeNull();
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
    expect(tray).toHaveClass('flex-none', 'overflow-y-auto');
    expect(tray).not.toHaveClass('fixed');
    expect(within(tray).getAllByRole('alertdialog')).toHaveLength(2);
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
