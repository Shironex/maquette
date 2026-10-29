import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { openMockGame } from '@/test/mock-ipc';
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
    expect(card).toHaveTextContent('3 models in total, about 90 credits');
    expect(card).toHaveTextContent('This spends about 90 Tripo credits');
    await userEvent.click(within(card).getByRole('button', { name: 'Approve and send 3' }));
    expect(calls).toContain('mcp_decide');
  });

  it("asks for an agent's OpenAI Images round with its dollar estimate", async () => {
    const calls = await openAgentSession('images');
    render(<ApprovalCard approval={approval('approval-3')} />);
    const card = screen.getByRole('alertdialog');
    expect(card).toHaveTextContent('Make 2 images for Frozen lake arena with OpenAI Images?');
    expect(card).toHaveTextContent('This spends about $0.13 from the OpenAI key');
    expect(card).toHaveTextContent('2 images at medium quality');
    await userEvent.click(within(card).getByRole('button', { name: 'Approve and make 2' }));
    expect(calls).toContain('mcp_decide');
  });

  it('asks to ship a run stopped before Ship', async () => {
    await openAgentSession('active');
    render(<ApprovalCard approval={approval('approval-1')} />);
    const card = screen.getByRole('alertdialog');
    expect(card).toHaveTextContent('Everything before Ship is done');
    expect(within(card).getByRole('button', { name: 'Approve and ship' })).toBeEnabled();
  });

  it('says a game ship replaces the imported originals and to commit first', async () => {
    await openMockGame('crypt');
    await useAgentsStore.getState().load();
    const ship = useAgentsStore.getState().status!.approvals.find(a => a.project === 'crypt-game')!;
    render(<ApprovalCard approval={ship} />);
    const card = screen.getByRole('alertdialog');
    expect(await within(card).findByText('2 replace the imported originals')).toBeInTheDocument();
    expect(card).toHaveTextContent(
      'The game repo has 3 files not committed yet. Commit them first, so the Ship change is ' +
        'easy to review and undo.'
    );
  });
});
