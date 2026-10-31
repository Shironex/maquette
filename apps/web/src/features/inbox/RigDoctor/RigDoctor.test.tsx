import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RigRecord } from '@/ipc/client';
import { creatureCase, oneIslandCase, refusedCase, stoppedCase } from '@/ipc/mock/rig-cases';
import { fixedCase, readCase, testedCase } from '@/ipc/mock/rig-fixtures';
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

  it('keeps the draft after two rounds and offers no way to add it', () => {
    show(stoppedCase());
    expect(screen.getByText('The draft is kept, not added.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Add as/ })).toBeNull();
    expect(screen.getByRole('button', { name: 'Discard draft' })).toBeInTheDocument();
  });
});
