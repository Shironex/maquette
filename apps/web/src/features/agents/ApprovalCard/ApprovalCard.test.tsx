import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type Approval } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useGripSaveStore } from '@/state/use-grip-save-store';
import { useRecipeSaveStore } from '@/state/use-recipe-save-store';
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
  it('tells the app which card answered and what pressed it', async () => {
    await openAgentSession('batch');
    const decide = vi.spyOn(commands, 'mcpDecide');
    const { unmount } = render(<ApprovalCard approval={approval('approval-2')} floating />);
    await userEvent.click(screen.getByRole('button', { name: 'Approve and send 3' }));
    expect(decide).toHaveBeenCalledWith(
      'approval-2',
      true,
      // jsdom's events are never trusted; a real click in the app is.
      expect.objectContaining({ source: 'floating-card', input: 'pointer', trusted: false })
    );
    unmount();
    await openAgentSession('images');
    render(<ApprovalCard approval={approval('approval-3')} />);
    screen.getByRole('button', { name: 'Deny' }).focus();
    await userEvent.keyboard('{Enter}');
    expect(decide).toHaveBeenLastCalledWith(
      'approval-3',
      false,
      expect.objectContaining({ source: 'agents-panel', input: 'keyboard' })
    );
  });

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

  it('names the folder and the command before a dev server of an agent-set repo runs', async () => {
    const calls = await openAgentSession('preview');
    render(<ApprovalCard approval={approval('approval-4')} />);
    const card = screen.getByRole('alertdialog', {
      name: 'Run the dev command of frostveil-site?',
    });
    expect(card).toHaveTextContent('/Users/you/code/frostveil-site');
    expect(card).toHaveTextContent('pnpm run dev --port {port}');
    expect(card).toHaveTextContent('astro dev');
    expect(card).toHaveTextContent('predev: node scripts/sync-models.js');
    expect(card).toHaveTextContent('including nested packages and the files scripts load');
    expect(card).not.toHaveTextContent('cannot tell');
    expect(card).toHaveTextContent("the site's package.json");
    await userEvent.click(within(card).getByRole('button', { name: 'Approve and run' }));
    expect(calls).toContain('mcp_decide');
    await waitFor(() => expect(useAgentsStore.getState().status?.approvals).toHaveLength(0));
  });

  it('says it cannot tell what a command runs, and names a file that can change it', async () => {
    await openAgentSession('preview');
    const known = approval('approval-4');
    const unknown: Approval = {
      ...known,
      preview: {
        ...known.preview!,
        command: 'corepack pnpm dev',
        scriptRuns: null,
        hooks: [],
        managerConfig: ['.yarnrc.yml'],
      },
    };
    render(<ApprovalCard approval={unknown} />);
    const card = screen.getByRole('alertdialog');
    expect(card).toHaveTextContent('Maquette cannot tell what this command runs.');
    expect(card).not.toHaveTextContent('Script runs');
    expect(card).toHaveTextContent('.yarnrc.yml');
  });

  it('denies a page preview without running anything', async () => {
    const calls = await openAgentSession('preview');
    render(<ApprovalCard approval={approval('approval-4')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Deny' }));
    await waitFor(() => expect(calls).toContain('mcp_decide'));
    expect(calls).not.toContain('page_server_start');
    expect(calls).not.toContain('site_preview');
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
    expect(card).toHaveTextContent('Writes to /Users/you/code/frostveil-site');
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

  it('opens an agent asked recipe bake in its review, and Deny keeps the bake', async () => {
    const showReview = vi.fn().mockResolvedValue(undefined);
    useRecipeBakesStore.setState({ showReview });
    const decide = vi.fn().mockResolvedValue(undefined);
    const kept = useAgentsStore.getState().decide;
    useAgentsStore.setState({ decide });
    const ask = {
      id: 'approval-recipe-1',
      kind: 'recipe-bake',
      project: 'frostveil',
      runId: null,
      runNumber: null,
      step: null,
      job: 'reach-for-the-lantern',
      assetName: 'Clay figure v3',
      service: null,
      detail: 'Reach for the lantern baked onto Clay figure v2. 2 numbers of the review warn.',
      items: [],
      requestedAtMs: 0,
      asset: 'clay-figure',
      version: 2,
    } satisfies Approval;
    render(<ApprovalCard approval={ask} />);
    expect(screen.getByRole('alertdialog', { name: 'Add Clay figure v3?' })).toBeInTheDocument();
    expect(screen.getByText(/2 numbers of the review warn/)).toBeVisible();
    expect(screen.queryByRole('button', { name: /Approve/ })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Review the bake' }));
    expect(showReview).toHaveBeenCalledWith('frostveil', 'reach-for-the-lantern');
    await userEvent.click(screen.getByRole('button', { name: 'Deny' }));
    expect(decide).toHaveBeenCalledWith('approval-recipe-1', false, expect.anything());
    useAgentsStore.setState({ decide: kept });
    useRecipeBakesStore.setState(useRecipeBakesStore.getInitialState(), true);
  });

  it('opens an agent proposed recipe save in the save sheet, never saving, and Deny turns it down', async () => {
    const show = vi.fn().mockResolvedValue(undefined);
    useRecipeSaveStore.setState({ show });
    const decide = vi.fn().mockResolvedValue(undefined);
    const kept = useAgentsStore.getState().decide;
    useAgentsStore.setState({ decide });
    const saved = vi.spyOn(commands, 'recipeLibrarySave');
    const ask = {
      id: 'recipe-save-approval-1',
      kind: 'recipe-save',
      project: 'frostveil',
      runId: null,
      runNumber: null,
      step: null,
      job: 'reach-for-the-lantern',
      assetName: 'Reach for the lantern',
      service: null,
      detail:
        'An agent proposes saving Reach for the lantern to the recipe library as r4 of Reach for the lantern. 2 lines of changes.',
      items: [],
      requestedAtMs: 0,
      recipeSave: { recipe: 'reach-for-the-lantern', entry: 'reach-for-the-lantern', revision: 4 },
    } satisfies Approval;
    const { rerender } = render(<ApprovalCard approval={ask} />);
    expect(
      screen.getByRole('alertdialog', { name: 'Save Reach for the lantern to the recipe library?' })
    ).toBeInTheDocument();
    expect(screen.getByText(/proposes saving Reach for the lantern/)).toBeVisible();
    expect(screen.getByText(/Nothing is saved until you look at what changes/)).toBeVisible();
    expect(screen.queryByRole('button', { name: /Approve/ })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(show).toHaveBeenCalledWith(
      'frostveil',
      'reach-for-the-lantern',
      'reach-for-the-lantern'
    );
    rerender(
      <ApprovalCard approval={{ ...ask, recipeSave: { ...ask.recipeSave, entry: null } }} />
    );
    await userEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(show).toHaveBeenLastCalledWith('frostveil', 'reach-for-the-lantern', null);
    await userEvent.click(screen.getByRole('button', { name: 'Deny' }));
    expect(decide).toHaveBeenCalledWith('recipe-save-approval-1', false, expect.anything());
    expect(saved).not.toHaveBeenCalled();
    useAgentsStore.setState({ decide: kept });
    useRecipeSaveStore.setState(useRecipeSaveStore.getInitialState(), true);
  });

  it('opens an agent proposed grip save in the save a grip sheet, never saving', async () => {
    const show = vi.fn().mockResolvedValue(undefined);
    useGripSaveStore.setState({ show });
    const saved = vi.spyOn(commands, 'gripLibrarySave');
    const grip = {
      layer: 'l-1',
      frame: 30,
      name: 'Scythe, staff grip',
      weapon: 'scythe',
      hands: { holds: 'right' as const, supports: 'left' as const },
      bones: Array.from({ length: 33 }, (_, i) => `bone${i}`),
      latest: 2,
      file: 'library/grips/scythe-staff-grip/r3.grip.json',
    };
    const ask = {
      id: 'grip-save-approval-1',
      kind: 'grip-save',
      project: 'frostveil',
      runId: null,
      runNumber: null,
      step: null,
      job: 'scythe-charge-held',
      assetName: 'Scythe, staff grip r3',
      service: null,
      detail: 'From the pose keys on both hands in Scythe charge, held, at frame 30.',
      items: [],
      requestedAtMs: 0,
      recipeSave: { recipe: 'scythe-charge-held', entry: 'scythe-staff-grip', revision: 3, grip },
    } satisfies Approval;
    render(<ApprovalCard approval={ask} />);
    expect(
      screen.getByRole('alertdialog', { name: 'Save a grip: Scythe, staff grip r3' })
    ).toBeInTheDocument();
    expect(screen.getByText(/From the pose keys on both hands/)).toBeVisible();
    expect(screen.getByText('library/grips/scythe-staff-grip · 33 bones · r2 to r3')).toBeVisible();
    expect(screen.getByText(/save this grip to the grip library/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Review' }));
    expect(show).toHaveBeenCalledWith('frostveil', 'scythe-charge-held', 'scythe-staff-grip', grip);
    expect(saved).not.toHaveBeenCalled();
    useGripSaveStore.setState(useGripSaveStore.getInitialState(), true);
  });
});
