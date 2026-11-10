import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ReplaceTarget } from '@/state/built-replace';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import ReplaceBuiltSheet from './ReplaceBuiltSheet';

/** Frostveil with Lantern rebuilt, ready to take a place (`swap`) or already in one (`swapped`). */
async function open(built: 'swap' | 'swapped', target: ReplaceTarget) {
  await openMockWorkspace('configured', undefined, undefined, undefined, undefined, built);
  await useBuiltStore.getState().load('frostveil');
  useBuiltStore.setState({ replacing: target });
  render(<ReplaceBuiltSheet project="frostveil" target={target} />);
  return screen.getByRole('dialog');
}

const asset = (id: string) =>
  useAppStore.getState().workspace?.project?.assets?.find(a => a.id === id);

afterEach(() => {
  clearMocks();
  useBuiltStore.setState({ project: null, items: [], adding: false, replacing: null });
  useAppStore.setState({ notice: null });
});

describe('ReplaceBuiltSheet', () => {
  it('shows what changes for the pair before anything does, then replaces', async () => {
    const sheet = await open('swap', { mode: 'replace', asset: null, built: 'lantern-rebuilt' });
    expect(sheet).toHaveAccessibleName('Use Lantern rebuilt in place of a model');
    const replace = within(sheet).getByRole('button', { name: 'Replace' });
    expect(replace).toHaveAttribute('aria-disabled', 'true');
    expect(sheet).toHaveTextContent('Pick the built asset and the model it replaces.');
    // Only models no script builds can be replaced.
    const takes = within(sheet).getByRole('combobox', { name: /Takes the place of/ });
    const names = within(takes)
      .getAllByRole('option')
      .map(o => o.textContent);
    expect(names).toContain('Crystal lantern');
    expect(names).not.toContain('Laptop');

    await userEvent.selectOptions(takes, 'Crystal lantern');
    const table = await within(sheet).findByRole('table');
    expect(table).toHaveTextContent('models/crystal_lantern_v2.glb');
    expect(table).toHaveTextContent('models/crystal_lantern_v3.glb');
    expect(within(sheet).getByRole('region', { name: 'Stays as it is' })).toHaveTextContent(
      'Layout placement: position 3.6, 0, 1.2'
    );
    // Reading the plan changed nothing.
    expect(asset('crystal-lantern')?.versions).toHaveLength(2);
    expect(replace).not.toHaveAttribute('aria-disabled');

    await userEvent.click(replace);
    await vi.waitFor(() => expect(useBuiltStore.getState().replacing).toBeNull());
    expect(asset('crystal-lantern')?.versions.at(-1)?.file).toBe('models/crystal_lantern_v3.glb');
    expect(asset('lantern-rebuilt')).toBeUndefined();
    const built = useBuiltStore.getState().items.find(b => b.asset === 'crystal-lantern');
    expect(built?.source.script).toBe('built/crystal_lantern.py');
    expect(built?.source.replaced?.by.name).toBe('Lantern rebuilt');
    expect(useAppStore.getState().notice?.title).toBe(
      "Lantern rebuilt is in Crystal lantern's place"
    );
  });

  it('opens from the model to replace and waits for the built asset', async () => {
    const sheet = await open('swap', { mode: 'replace', asset: 'crystal-lantern', built: null });
    expect(sheet).toHaveAccessibleName('Replace with a built asset');
    expect(within(sheet).queryByRole('table')).toBeNull();
    await userEvent.selectOptions(
      within(sheet).getByRole('combobox', { name: /Built asset/ }),
      'Lantern rebuilt'
    );
    expect(await within(sheet).findByRole('table')).toBeInTheDocument();
    expect(sheet).toHaveAccessibleName('Use Lantern rebuilt in place of a model');
  });

  it('says why a pair cannot be swapped and keeps Replace off', async () => {
    const sheet = await open('swap', {
      mode: 'replace',
      asset: 'crystal-lantern',
      built: 'newtons-cradle',
    });
    // The read's spinner is a status too, so the banner is found by its title.
    await within(sheet).findByText('This pair cannot be swapped yet');
    expect(within(sheet).getByRole('status')).toHaveTextContent(
      'Newtons cradle has a script that changed since its last build. Rebuild it first'
    );
    expect(within(sheet).getByRole('button', { name: 'Replace' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(within(sheet).queryByRole('table')).toBeNull();
  });

  it('brings the old model back for an asset a built model replaced', async () => {
    const sheet = await open('swapped', { mode: 'undo', asset: 'crystal-lantern' });
    expect(sheet).toHaveAccessibleName('Bring back the old model of Crystal lantern');
    expect(within(sheet).queryByRole('combobox')).toBeNull();
    expect(await within(sheet).findByRole('region', { name: 'Current again' })).toHaveTextContent(
      'v2 models/crystal_lantern_v2.glb'
    );
    expect(within(sheet).getByRole('region', { name: 'Leaves the asset' })).toHaveTextContent(
      'v3 models/crystal_lantern_v3.glb'
    );
    expect(asset('lantern-rebuilt')).toBeUndefined();

    await userEvent.click(within(sheet).getByRole('button', { name: 'Bring it back' }));
    await vi.waitFor(() => expect(useBuiltStore.getState().replacing).toBeNull());
    expect(asset('crystal-lantern')?.versions.at(-1)?.file).toBe('models/crystal_lantern_v2.glb');
    expect(asset('lantern-rebuilt')?.name).toBe('Lantern rebuilt');
    expect(useBuiltStore.getState().items.map(b => b.asset)).toContain('lantern-rebuilt');
    expect(useBuiltStore.getState().items.map(b => b.asset)).not.toContain('crystal-lantern');
    expect(useAppStore.getState().notice?.title).toBe('Crystal lantern is back on v2');
  });

  it('closes on Cancel and changes nothing', async () => {
    const sheet = await open('swap', { mode: 'replace', asset: null, built: 'lantern-rebuilt' });
    await userEvent.click(within(sheet).getByRole('button', { name: 'Cancel' }));
    expect(useBuiltStore.getState().replacing).toBeNull();
    expect(asset('lantern-rebuilt')?.name).toBe('Lantern rebuilt');
  });
});
