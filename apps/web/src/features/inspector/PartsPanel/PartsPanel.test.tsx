import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import { PartsPanel } from '.';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

beforeEach(async () => {
  await openMockParts();
});
afterEach(() => clearMocks());

const list = () => screen.getByRole('list', { name: 'Parts' });
const names = () =>
  within(list())
    .getAllByRole('button')
    .filter(b => b.hasAttribute('aria-pressed'))
    .map(b => b.textContent);

describe('PartsPanel', () => {
  it('lists the split lantern by the names it ships as', async () => {
    render(<PartsPanel />);
    await waitFor(() => expect(list()).toBeInTheDocument());
    expect(names()).toEqual(['lid', 'cage', 'crystal', 'post', 'base']);
    expect(screen.getByText('5 parts · split in Blender')).toBeInTheDocument();
    expect(screen.getByText('lid, cage, crystal, post, base')).toBeInTheDocument();
  });

  it('renames a part on double click and saves it', async () => {
    render(<PartsPanel />);
    await waitFor(() => expect(list()).toBeInTheDocument());
    await userEvent.dblClick(within(list()).getByRole('button', { name: 'post' }));
    const field = screen.getByRole('textbox', { name: 'Rename post' });
    await userEvent.clear(field);
    await userEvent.type(field, 'pillar{Enter}');
    await waitFor(() => expect(names()).toContain('pillar'));
    const saved = await commands.partsGet('frostveil', 'frost-lantern');
    expect(saved.plan.names?.part_4).toBe('pillar');
  });

  it('refuses two parts under one name', async () => {
    render(<PartsPanel />);
    await waitFor(() => expect(list()).toBeInTheDocument());
    await userEvent.dblClick(within(list()).getByRole('button', { name: 'post' }));
    const field = screen.getByRole('textbox', { name: 'Rename post' });
    await userEvent.clear(field);
    await userEvent.type(field, 'lid{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'part_1 and part_4 would both ship as lid.'
    );
  });

  it('merges two selected parts and hides one', async () => {
    render(<PartsPanel />);
    await waitFor(() => expect(list()).toBeInTheDocument());
    await userEvent.click(within(list()).getByRole('button', { name: 'post' }));
    expect(screen.getByText('1 selected')).toBeInTheDocument();
    // Shift adds to the selection.
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(within(list()).getByRole('button', { name: 'base' }));
    await user.keyboard('{/Shift}');
    expect(screen.getByText('2 selected')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Merge' }));
    await waitFor(() => expect(within(list()).getAllByRole('listitem')).toHaveLength(4));
    expect(screen.getByText('+ part_5')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Hide' }));
    expect(await screen.findByRole('button', { name: 'Show' })).toBeInTheDocument();
  });

  it('offers a split for a model that is one mesh', async () => {
    render(<PartsPanel />);
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Selected model' }),
      'Rune chest'
    );
    expect(await screen.findByText('One mesh, no parts')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Split into parts' }));
    await waitFor(() => expect(list()).toBeInTheDocument());
    const prep = await commands.prepareAsset('frostveil', 'rune-chest');
    expect(prep.entry.fixups).toEqual([{ op: 'split-parts', by: 'loose', mergeUnder: 0.01 }]);
  });
});
