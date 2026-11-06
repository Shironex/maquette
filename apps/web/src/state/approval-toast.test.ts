import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type Approval, type ApprovalKind, type McpStatus } from '@/ipc/client';
import { approvalToast } from './approval-toast';
import { decidedBy } from './decisions';
import { useAgentsStore } from './use-agents-store';
import { useAppStore } from './use-app-store';

const base: Approval = {
  id: 'approval-1',
  kind: 'ship',
  project: 'frostveil',
  runId: 'frostveil-run-26',
  runNumber: 26,
  step: 'Ship',
  job: null,
  assetName: null,
  service: null,
  detail: null,
  items: [],
  requestedAtMs: 0,
};

const ask = (kind: ApprovalKind, more: Partial<Approval> = {}): Approval => ({
  ...base,
  kind,
  runId: kind === 'ship' ? base.runId : null,
  runNumber: kind === 'ship' ? base.runNumber : null,
  ...more,
});

const KINDS: ApprovalKind[] = [
  'ship',
  'send-to-3d',
  'image-round',
  'rest-pose',
  'reweight',
  'springs',
  'rig-doctor',
  'recipe-bake',
  'recipe-save',
  'page-preview',
];

describe('approvalToast', () => {
  it('never talks about a run or Ship for a kind that is not a ship', () => {
    for (const kind of KINDS.filter(k => k !== 'ship')) {
      for (const approve of [true, false]) {
        const { title, detail } = approvalToast(ask(kind), approve, false);
        expect(`${title} ${detail}`).not.toMatch(/\bRun\b|Ship|null|undefined/);
      }
    }
  });

  it('says a denied recipe save was not saved and that the agent is told', () => {
    const recipe = ask('recipe-save', { assetName: 'Reach for the lantern' });
    expect(approvalToast(recipe, false, false)).toEqual({
      status: 'info',
      title: 'Not saved to the library',
      detail: 'The library did not change. The agent is told it was denied.',
    });
    expect(approvalToast(recipe, true, false).title).toBe(
      'Reach for the lantern was saved to the library'
    );
  });

  it('says a denied recipe bake was not added and that the agent is told', () => {
    const bake = ask('recipe-bake', { assetName: 'Reach v3' });
    expect(approvalToast(bake, false, false)).toEqual({
      status: 'info',
      title: 'Not added',
      detail: 'The bake was not added as a version. The agent is told it was denied.',
    });
    expect(approvalToast(bake, true, false)).toMatchObject({
      status: 'done',
      title: 'Reach v3 was added',
    });
  });

  it('keeps the Ship words for a ship, with the game repo note for a game', () => {
    expect(approvalToast(base, true, true)).toEqual({
      status: 'done',
      title: 'Run 26 is shipping',
      detail: 'Ship never commits. Review the change in git and commit it yourself.',
    });
    expect(approvalToast(base, false, false)).toEqual({
      status: 'info',
      title: 'Run 26 stopped',
      detail: 'Ship did not run. The agent is told it was denied.',
    });
  });

  it('names the asset and the service of a send', () => {
    const send = ask('send-to-3d', { assetName: 'Guardian statue', service: 'Tripo' });
    expect(approvalToast(send, true, false).title).toBe('Guardian statue is on its way to Tripo');
    expect(approvalToast(send, false, false).title).toBe('Guardian statue was not sent');
  });
});

describe('useAgentsStore.decide', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    useAgentsStore.setState(useAgentsStore.getInitialState(), true);
    useAppStore.setState(useAppStore.getInitialState(), true);
  });

  it('toasts the recipe save words when a recipe save is denied', async () => {
    const recipe = ask('recipe-save', { id: 'recipe-save-1', assetName: 'Reach' });
    const status = { approvals: [recipe] } as unknown as McpStatus;
    useAgentsStore.setState({ status });
    vi.spyOn(commands, 'mcpDecide').mockResolvedValue({ ...status, approvals: [] });
    const notify = vi.fn();
    useAppStore.setState({ notify });
    await useAgentsStore.getState().decide('recipe-save-1', false, decidedBy('agents-panel'));
    expect(notify).toHaveBeenCalledWith({
      status: 'info',
      title: 'Not saved to the library',
      detail: 'The library did not change. The agent is told it was denied.',
    });
  });
});
