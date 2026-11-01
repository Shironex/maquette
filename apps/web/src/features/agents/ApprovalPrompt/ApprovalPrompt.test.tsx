import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { act } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { openAgentSession } from '../openAgentSession';
import { ApprovalPrompt } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
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

  it('holds the bottom right corner, clear of the views header controls', async () => {
    await openAgentSession('batch');
    act(() => useAppStore.setState({ view: 'board' }));
    const { container } = render(<ApprovalPrompt />);
    const column = container.firstElementChild;
    expect(column).toHaveClass('bottom-11', 'right-5', 'overflow-y-auto');
    expect(column).not.toHaveClass('top-[68px]');
  });

  it('renders nothing with no approvals', async () => {
    await openAgentSession(null);
    const { container } = render(<ApprovalPrompt />);
    expect(container).toBeEmptyDOMElement();
  });
});
