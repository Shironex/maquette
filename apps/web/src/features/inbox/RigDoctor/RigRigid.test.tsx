import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type RigRecord } from '@/ipc/client';
import { rigidSeeds } from '@/ipc/mock/rig-rigid';
import { fixedCase } from '@/ipc/mock/rig-fixtures';
import { useRigStore } from '@/state/use-rig-store';
import { RigDoctor } from '.';

function show(record: RigRecord) {
  useRigStore.setState({ record, busy: false });
  return render(<RigDoctor record={record} />);
}

describe('RigRigid', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useRigStore.setState({ record: null, open: [], busy: false });
  });
  afterEach(cleanup);

  it('offers no rigid parts when the arm bones hold none', () => {
    show(fixedCase('a-v1', 'A'));
    expect(screen.queryByRole('region', { name: 'Rigid parts' })).toBeNull();
  });

  it('binds the picked parts to Spine2 by default, or to the bone a person picks', async () => {
    const c = rigidSeeds.rigid();
    const bind = vi.spyOn(commands, 'rigBindRigid').mockResolvedValue(c);
    show(c);
    expect(screen.getByText('Part 1, left')).toBeInTheDocument();
    expect(screen.getByLabelText('Follows')).toHaveValue('mixamorig:Spine2');
    await userEvent.click(screen.getByText('Part 2, right'));
    await userEvent.click(screen.getByRole('button', { name: 'Bind rigidly' }));
    expect(bind).toHaveBeenLastCalledWith(c.id, [1], 'mixamorig:Spine2');
    await userEvent.selectOptions(screen.getByLabelText('Follows'), 'mixamorig:Hips');
    await userEvent.click(screen.getByRole('button', { name: 'Bind rigidly' }));
    expect(bind).toHaveBeenLastCalledWith(c.id, [1], 'mixamorig:Hips');
  });

  it('shows an agents bind with its stretch before and after, for a person to answer', async () => {
    const c = rigidSeeds['rigid-asked']();
    const decide = vi.spyOn(commands, 'rigDecideRigid').mockResolvedValue(c);
    show(c);
    const card = screen.getByRole('alertdialog', { name: 'Use the bound parts?' });
    expect(card).toHaveTextContent('An agent asked for this.');
    expect(card).toHaveTextContent(/worst stretch 275 to 41 mm/);
    expect(screen.getByRole('list', { name: 'Rows the bind moved' })).toHaveTextContent(
      'raise57 to 16 mm, better'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Use bound parts' }));
    expect(decide).toHaveBeenCalledWith(c.id, true);
  });

  it('will not use a bind whose checks did not hold', () => {
    const c = rigidSeeds['rigid-asked']();
    const check = { ok: false, title: 'Mesh changed', detail: 'positions moved', value: '' };
    show({ ...c, rigid: { ...c.rigid!, checks: [check] } });
    expect(screen.getByRole('button', { name: 'Use bound parts' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByText('Mesh changed: positions moved')).toBeInTheDocument();
  });

  it('undoes a bind in the draft', async () => {
    const c = rigidSeeds['rigid-bound']();
    const decide = vi.spyOn(commands, 'rigDecideRigid').mockResolvedValue(c);
    show(c);
    expect(screen.getByText('Parts 1 and 2 follow Spine2 in the draft')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Undo the bind' }));
    expect(decide).toHaveBeenCalledWith(c.id, false);
  });
});
