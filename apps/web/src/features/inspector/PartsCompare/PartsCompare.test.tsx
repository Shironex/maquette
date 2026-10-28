import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import { PartsCompare } from '.';

vi.mock('../CompareCanvas', () => ({ default: () => null }));

beforeEach(async () => {
  await openMockParts();
});
afterEach(() => clearMocks());

async function lantern() {
  const found = await commands.partsPreview('frostveil', 'frost-lantern');
  const asset = useAppStore
    .getState()
    .workspace!.project!.assets!.find(a => a.id === 'frost-lantern')!;
  return { asset, found };
}

const row = (label: string) =>
  within(screen.getByRole('table', { name: 'Before and after' }))
    .getByRole('rowheader', { name: label })
    .closest('tr')!;

describe('PartsCompare', () => {
  it('shows the lantern as it came against after its split', async () => {
    const { asset, found } = await lantern();
    render(<PartsCompare asset={asset} found={found} onChanged={vi.fn()} />);
    await waitFor(() => expect(row('Draw calls')).toHaveTextContent('45'));
    expect(row('Parts Motion can move')).toHaveTextContent('nonelid, cage, crystal, post, base');
    expect(within(row('Parts Motion can move')).getByRole('img', { name: 'better' })).toBeVisible();
    expect(row('Inbox checks')).toHaveTextContent('5 pass, 1 warning6 pass');
    expect(screen.getAllByText('v2 · split in Blender, 5 parts')).toHaveLength(2);
  });

  it('keeps the model as it came by dropping the split', async () => {
    const { asset, found } = await lantern();
    const onChanged = vi.fn();
    render(<PartsCompare asset={asset} found={found} onChanged={onChanged} />);
    await userEvent.click(screen.getByRole('button', { name: 'Keep v2 as it came' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalled());
    const prep = await commands.prepareAsset('frostveil', 'frost-lantern');
    expect(prep.entry.fixups).toEqual([]);
    expect(useAppStore.getState().notice?.title).toBe('Frost lantern stays one mesh');
  });

  it('keeps an older version by making it current', async () => {
    const { asset } = await lantern();
    await commands.prepareSave('frostveil', 'frost-lantern', {
      size: null,
      fixups: [],
      skipBake: null,
      padEdges: null,
    });
    const found = await commands.partsGet('frostveil', 'frost-lantern');
    render(<PartsCompare asset={asset} found={found} onChanged={vi.fn()} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Keep v1' }));
    await waitFor(() =>
      expect(
        useAppStore
          .getState()
          .workspace!.project!.assets!.find(a => a.id === 'frost-lantern')!
          .versions.at(-1)?.version
      ).toBe(1)
    );
  });
});
