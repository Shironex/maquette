import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Approval } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { useRigStore } from '@/state/use-rig-store';
import { openAvatar } from '@/test/open-avatar';
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

  it('opens the Avatar view on the version a rest pose waits on', async () => {
    await openAvatar('older');
    const pose = useAgentsStore.getState().status!.approvals.find(a => a.kind === 'rest-pose')!;
    expect(pose.version).toBe(2);
    render(<ApprovalCard approval={pose} />);
    await userEvent.click(screen.getByRole('button', { name: 'Open the sheets' }));
    expect(useAvatarStore.getState()).toMatchObject({ asset: 'elsheria', version: 2 });
    await waitFor(() => expect(useAvatarStore.getState().view?.normalized?.state).toBe('pending'));
  });

  it("opens an agent's rest pose in the Avatar view, or turns it down", async () => {
    const calls = await openAvatar('pending');
    const pose = useAgentsStore.getState().status!.approvals.find(a => a.kind === 'rest-pose')!;
    render(<ApprovalCard approval={pose} />);
    const card = screen.getByRole('alertdialog');
    expect(card).toHaveTextContent("Approve Elsheria's new rest pose?");
    expect(card).toHaveTextContent('lifted left arm 55');
    expect(within(card).queryByRole('button', { name: /Approve/ })).not.toBeInTheDocument();
    await userEvent.click(within(card).getByRole('button', { name: 'Open the sheets' }));
    expect(useAppStore.getState().view).toBe('avatar');
    expect(useAvatarStore.getState().asset).toBe('elsheria');
    await userEvent.click(within(card).getByRole('button', { name: 'Deny' }));
    await waitFor(() => expect(calls).toContain('avatar_decide'));
    expect(calls).not.toContain('mcp_decide');
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

  it('opens the rig doctor for an agent fix and has no way to add it from the card', async () => {
    const showCase = vi.fn().mockResolvedValue(undefined);
    useRigStore.setState({ showCase });
    const rig = {
      id: 'approval-rig-1',
      kind: 'rig-doctor',
      project: 'frostveil',
      runId: null,
      runNumber: null,
      step: null,
      job: 'spiked-hound-v1',
      assetName: 'Spiked hound v2',
      service: null,
      detail: 'The rig doctor fixed 2 defects.',
      items: [],
      requestedAtMs: 0,
      estimate: null,
      conceptJobs: [],
      estimateUsd: null,
      asset: 'spiked-hound',
      version: 1,
      sheets: null,
    } satisfies Approval;
    render(<ApprovalCard approval={rig} />);
    expect(screen.getByRole('alertdialog', { name: 'Add Spiked hound v2?' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Approve/ })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Open the sheets' }));
    expect(showCase).toHaveBeenCalledWith('spiked-hound-v1');
  });
});
