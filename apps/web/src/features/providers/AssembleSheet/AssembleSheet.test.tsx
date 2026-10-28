import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useSplitStore } from '@/state/use-split-store';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import { AssembleSheet } from '.';

beforeEach(async () => {
  await openMockParts();
});
afterEach(() => {
  useSplitStore.setState({ assemble: null });
  clearMocks();
});

describe('AssembleSheet', () => {
  it('joins three part models into one prop with a hinge for the door', async () => {
    useSplitStore.setState({ assemble: { project: 'frostveil', parts: ['sigil-gate-frame'] } });
    render(<AssembleSheet />);
    const add = screen.getByRole('combobox', { name: 'Add a part' });
    await userEvent.selectOptions(add, 'Sigil gate door');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Add a part' }),
      'Sigil gate steps'
    );
    expect(screen.getByText('Give the prop a name of 1 to 64 characters.')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Prop name' }), 'sigil_gate');
    const door = screen.getByRole('textbox', { name: 'Node name for Sigil gate door' });
    await userEvent.clear(door);
    await userEvent.type(door, 'door_leaf');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Pivot of door_leaf' }),
      'Left hinge'
    );
    expect(screen.getByText('door_leaf')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Assemble' }));
    await waitFor(() => expect(useSplitStore.getState().assemble).toBeNull());
    const gate = useAppStore
      .getState()
      .workspace?.project?.assets?.find(a => a.id === 'sigil-gate');
    expect(gate?.versions).toHaveLength(1);
    const recipe = await commands.assembleRecipe('frostveil', 'sigil-gate');
    expect(recipe?.parts.map(p => [p.name, p.pivot])).toEqual([
      ['sigil_gate_frame', 'bottom-centre'],
      ['door_leaf', 'left-hinge'],
      ['sigil_gate_steps', 'bottom-centre'],
    ]);
  });
});
