import { cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RigRecord } from '@/ipc/client';
import {
  creatureCase,
  creatureFixedCase,
  oneIslandCase,
  refusedCase,
  stoppedCase,
} from '@/ipc/mock/rig-cases';
import { fixedCase, readCase, testedCase } from '@/ipc/mock/rig-fixtures';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRigStore } from '@/state/use-rig-store';
import { RigDoctor } from '.';

function show(record: RigRecord) {
  useRigStore.setState({ record, busy: false });
  return render(<RigDoctor record={record} />);
}

describe('RigDoctor', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useRigStore.setState({ record: null, open: [], busy: false });
  });

  it('shows the before sheet with defects by place and asks for the click that fixes', async () => {
    const run = vi.spyOn(commands, 'rigRun').mockResolvedValue(fixedCase('a-v1', 'A'));
    show(testedCase('a-v1', 'A'));
    expect(screen.getByRole('heading', { name: 'Defects by place' })).toBeInTheDocument();
    expect(screen.getByText('Cape weighted to the arm bones')).toBeInTheDocument();
    expect(screen.getByText('Needs your yes')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Run the fix' }));
    expect(run).toHaveBeenCalledWith('a-v1', 'fix');
  });

  it('offers no run before the model is accepted as v1', () => {
    show(readCase('a-inbox', 'A', 'a.glb'));
    expect(screen.getByRole('button', { name: 'Accept as v1' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Run the pose test' })).toBeNull();
  });

  it('asks about the joint move on a card that names the clips it would break', async () => {
    const decide = vi.spyOn(commands, 'rigDecideJoints').mockResolvedValue(fixedCase('a-v1', 'A'));
    show(fixedCase('a-v1', 'A'));
    const card = screen.getByRole('alertdialog', { name: 'Also move the left arm joints?' });
    expect(card).toHaveTextContent('This would break walk');
    await userEvent.click(screen.getByRole('button', { name: 'Keep weights only' }));
    expect(decide).toHaveBeenCalledWith('a-v1', false);
  });

  it('adds the draft only through Add as v2, after the checks', async () => {
    const land = vi.spyOn(commands, 'rigLand').mockResolvedValue(fixedCase('a-v1', 'A'));
    show(fixedCase('a-v1', 'A'));
    expect(screen.getByText('Only JOINTS_0 and WEIGHTS_0 changed')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Add as v2' }));
    expect(land).toHaveBeenCalledWith('a-v1');
  });

  it('says a one island mesh gets the narrow reweight and has no second round', () => {
    show({ ...fixedCase('a-v1', 'A'), model: oneIslandCase().model });
    expect(screen.getByText('One continuous mesh')).toBeInTheDocument();
    expect(screen.getByText(/narrow reweight moves only/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Round 2' })).toBeDisabled();
  });

  it('shows a creature run with its chains, map, log and a disabled Add', () => {
    show(creatureCase());
    expect(screen.getByRole('cell', { name: 'dup_FL' })).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Pose test after' })).toHaveAttribute(
      'aria-valuenow',
      '9'
    );
    expect(screen.getByText('Started over MCP')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add as v2' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Cancel run' })).toBeEnabled();
  });

  it('says why it refuses a model and leaves Accept as is one click away', () => {
    show(refusedCase());
    expect(
      screen.getByRole('heading', { name: 'The rig doctor cannot fix Crystal golem' })
    ).toBeInTheDocument();
    expect(screen.getByText('Three skinned meshes')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Accept as is' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Run the fix' })).toBeNull();
  });

  it('opens the redo flow from a refused model, and spends nothing itself', async () => {
    const openRedo = vi.fn();
    useProvidersStore.setState({ openRedo });
    const link = {
      job: 'veil-gate-2',
      project: 'frostveil',
      asset: 'veil-gate',
      assetName: 'Veil gate',
      provider: 'meshy' as const,
    };
    useInboxStore.setState({
      snapshot: { items: [{ id: 'veil_gate.glb', link }] } as never,
    });
    show(refusedCase());
    await userEvent.click(screen.getByRole('button', { name: 'Redo...' }));
    expect(openRedo).toHaveBeenCalledWith('frostveil', 'veil-gate', 'veil-gate-2');
    useInboxStore.setState({ snapshot: null });
  });

  it('cannot redo a refused model that came from no send', () => {
    useInboxStore.setState({ snapshot: null });
    show(refusedCase());
    expect(screen.getByRole('button', { name: 'Redo...' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('picks the other route, but a model with its own names has no humanoid', async () => {
    const set = vi.spyOn(commands, 'rigSetRoute').mockResolvedValue(readCase('a', 'A', null));
    show(testedCase('a-v1', 'A'));
    const route = screen.getByRole('group', { name: 'Route' });
    await userEvent.click(within(route).getByRole('button', { name: 'Creature' }));
    expect(set).toHaveBeenCalledWith('a-v1', 'creature');
    cleanup();
    show({ ...testedCase('b-v1', 'B'), route: 'creature', model: creatureCase().model });
    const own = screen.getByRole('group', { name: 'Route' });
    expect(within(own).getByRole('button', { name: 'Humanoid' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('holds the route while a draft is there', () => {
    show(fixedCase('a-v1', 'A'));
    const route = screen.getByRole('group', { name: 'Route' });
    expect(within(route).getByRole('button', { name: 'Creature' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('waits for a person to look at a creatures chains before adding the draft', async () => {
    const confirm = vi
      .spyOn(commands, 'rigConfirmChains')
      .mockResolvedValue({ ...creatureFixedCase(), chainsConfirmed: true });
    show(creatureFixedCase());
    expect(screen.getByText('Not checked yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add as v2' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Chains are right' }));
    expect(confirm).toHaveBeenCalledWith('spiked-hound-v1', []);
    await vi.waitFor(() => expect(useRigStore.getState().record?.chainsConfirmed).toBe(true));
  });

  it('edits a chain: its side and what it reads as, and saying so starts over', async () => {
    const confirm = vi
      .spyOn(commands, 'rigConfirmChains')
      .mockResolvedValue({ ...creatureFixedCase(), chainsConfirmed: true });
    show(creatureFixedCase());
    await userEvent.click(screen.getByRole('button', { name: 'Edit' }));
    // Rows that hold several chains cannot be edited.
    expect(screen.queryByRole('combobox', { name: 'Read body as' })).toBeNull();
    expect(screen.queryByRole('combobox', { name: 'Side of tail' })).toBeNull();
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Read tail as' }), 'junk');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Side of leg_FL' }),
      'right'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Save and start over' }));
    expect(confirm).toHaveBeenCalledWith('spiked-hound-v1', [
      { key: 'bone_20', readAs: 'junk', side: null },
      { key: 'bone_1', readAs: null, side: 'right' },
    ]);
  });

  it('runs the walk proof on a creatures draft only, and never counts it as a round', async () => {
    const run = vi.spyOn(commands, 'rigRun').mockResolvedValue(creatureFixedCase());
    show(creatureFixedCase());
    await userEvent.click(screen.getByRole('button', { name: 'Run the proof' }));
    expect(run).toHaveBeenCalledWith('spiked-hound-v1', 'walk-proof');
    cleanup();
    show(fixedCase('a-v1', 'A'));
    expect(screen.queryByRole('button', { name: 'Run the proof' })).toBeNull();
  });

  it('keeps the draft after two rounds and offers no way to add it', () => {
    show(stoppedCase());
    expect(screen.getByText('The draft is kept, not added.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Add as/ })).toBeNull();
    expect(screen.getByRole('button', { name: 'Discard draft' })).toBeInTheDocument();
  });
});
