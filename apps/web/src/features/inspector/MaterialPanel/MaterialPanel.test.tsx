import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands, type InspectModel } from '@/ipc/client';
import { useMaterialsStore } from '@/state/use-materials-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import MaterialPanel from './MaterialPanel';

let character: InspectModel;
let shrubs: InspectModel;

beforeEach(async () => {
  await openMockWorkspace();
  const view = await commands.inspectProject('moon-island');
  character = view.models.find(m => m.model === 'character')!;
  shrubs = view.models.find(m => m.model === 'flower_shrubs')!;
  await useMaterialsStore.getState().load('moon-island');
});
afterEach(() => {
  clearMocks();
  useMaterialsStore.setState({ root: null, saved: {}, edits: {}, failed: {}, saving: false });
});

const edits = () => useMaterialsStore.getState().edits['moon-island']?.character;

describe('the material panel', () => {
  it('edits every material, marks the edit and applies it', async () => {
    render(<MaterialPanel project="moon-island" model={character} />);
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByRole('button', { name: 'Apply tweaks' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );

    const hex = screen.getByRole('textbox', { name: 'Tint hex' });
    await userEvent.type(hex, '6f7446{Enter}');
    expect(edits()?.all.tint).toBe('#6f7446');
    expect(screen.getByRole('status')).toHaveTextContent('Not applied yet');

    const roughness = screen.getByRole('group', { name: 'Roughness' });
    await userEvent.click(within(roughness, 'Set'));
    expect(edits()?.all.roughness).toEqual({ mode: 'override', value: 0.8 });
    expect(screen.getByText(/replaces the metal and roughness map/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Apply tweaks' }));
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
    const stored = await commands.materialsGet('moon-island');
    expect(stored.models.character?.all.tint).toBe('#6f7446');
  });

  it('edits one material by name on top of all of them', async () => {
    render(<MaterialPanel project="moon-island" model={character} />);
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Applies to' }),
      'tripo_mat_90739555.001'
    );
    await userEvent.click(within(screen.getByRole('group', { name: 'Metalness' }), 'Set'));
    expect(edits()?.materials['tripo_mat_90739555.001']?.metalness).toBe(0);
    expect(edits()?.all.metalness).toBeNull();
  });

  it('resets an edit, then clears what is saved', async () => {
    await commands.materialsSave('moon-island', {
      character: {
        all: { tint: '#5d6266', roughness: null, metalness: null, emissiveStrength: null },
        materials: {},
      },
    });
    await useMaterialsStore.getState().load('moon-island');
    render(<MaterialPanel project="moon-island" model={character} />);
    expect(screen.getByRole('textbox', { name: 'Tint hex' })).toHaveValue('#5d6266');

    await userEvent.click(screen.getByRole('button', { name: 'Clear the tint' }));
    expect(edits()?.all.tint).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Reset' }));
    expect(edits()).toBeUndefined();

    await userEvent.click(screen.getByRole('button', { name: 'Clear tweaks' }));
    expect(edits()).toEqual({
      all: { tint: null, roughness: null, metalness: null, emissiveStrength: null },
      materials: {},
    });
  });

  it('waits for the saved tweaks before Apply, and says why when they did not load', async () => {
    useMaterialsStore.setState({ saved: {} });
    render(<MaterialPanel project="moon-island" model={character} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Tint hex' }), '6f7446{Enter}');
    const apply = screen.getByRole('button', { name: 'Apply tweaks' });
    expect(apply).toHaveAttribute('aria-disabled', 'true');
    await userEvent.hover(apply);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Loading the saved tweaks');

    act(() => useMaterialsStore.setState({ failed: { 'moon-island': 'The file is broken.' } }));
    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      /did not load\. The file is broken/
    );

    await act(() => useMaterialsStore.getState().load('moon-island'));
    expect(screen.getByRole('button', { name: 'Apply tweaks' })).not.toHaveAttribute(
      'aria-disabled'
    );
  });

  it('says so when no asset makes the file', () => {
    render(<MaterialPanel project="moon-island" model={shrubs} />);
    expect(screen.getByText(/no asset makes this file/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Apply tweaks' })).toBeNull();
  });
});

function within(group: HTMLElement, name: string): HTMLElement {
  const button = [...group.querySelectorAll('button')].find(b => b.textContent === name);
  if (!button) throw new Error(`No ${name} button`);
  return button;
}
