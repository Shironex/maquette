import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { openAgentSession } from '../openAgentSession';
import { ApprovalCard } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null, deciding: null });
});

function approval(id: string) {
  return useAgentsStore.getState().status!.approvals.find(a => a.id === id)!;
}

describe('an approval card', () => {
  it('lists a batch send and approves it through the server', async () => {
    const calls = await openAgentSession('batch');
    render(<ApprovalCard approval={approval('approval-2')} />);
    const card = screen.getByRole('alertdialog');
    expect(within(card).getAllByRole('listitem')).toHaveLength(3);
    expect(card).toHaveTextContent('3 models in total, one model per asset');
    await userEvent.click(within(card).getByRole('button', { name: 'Approve and send 3' }));
    expect(calls).toContain('mcp_decide');
  });

  it('asks to ship a run stopped before Ship', async () => {
    await openAgentSession('active');
    render(<ApprovalCard approval={approval('approval-1')} />);
    const card = screen.getByRole('alertdialog');
    expect(card).toHaveTextContent('Everything before Ship is done');
    expect(within(card).getByRole('button', { name: 'Approve and ship' })).toBeEnabled();
  });
});
