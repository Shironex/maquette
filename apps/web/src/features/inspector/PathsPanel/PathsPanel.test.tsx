import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePathsStore } from '@/state/use-paths-store';
import { clearMocks, openPaths } from '@/test/open-paths';
import PathsPanel from './PathsPanel';

afterEach(clearMocks);

const draft = () => usePathsStore.getState().draft;

async function panel() {
  const s = await openPaths();
  return render(<PathsPanel view={s.view!} />);
}

describe('the paths panel', () => {
  it('lists every path with its length, fill and size', async () => {
    await panel();
    const list = screen.getAllByRole('list')[0]!;
    expect(within(list).getAllByRole('button')).toHaveLength(3);
    expect(within(list).getByText(/0% at Veil gate, 100% at Keystone/)).toBeInTheDocument();
    expect(within(list).getByText(/unlit/)).toBeInTheDocument();
    await userEvent.click(within(list).getByRole('button', { name: /Root glow/ }));
    expect(usePathsStore.getState().selection).toEqual({ path: 'root-glow', point: null });
  });

  it("changes the selected path's look", async () => {
    await panel();
    const width = screen.getByRole('textbox', { name: 'Width' });
    await userEvent.clear(width);
    await userEvent.type(width, '4{Enter}');
    expect(draft().paths[0]!.width).toBeCloseTo(0.04);
    await userEvent.click(screen.getByRole('button', { name: 'Straight' }));
    expect(draft().paths[0]!.curve).toBe('polyline');
    const hex = screen.getByRole('textbox', { name: 'Glow hex' });
    await userEvent.clear(hex);
    await userEvent.type(hex, '#00ff88{Enter}');
    expect(draft().paths[0]!.glow).toBe('#00ff88');
    expect(screen.getByRole('combobox', { name: 'Ends' })).toBeDisabled();
  });

  it('lets an end go of its model', async () => {
    await panel();
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'End follows' }), '');
    expect(draft().paths[0]!.end).toBeNull();
  });

  it('turns energize off and on, and opens the Timeline', async () => {
    await panel();
    const toggle = screen.getByRole('switch', { name: /Light it as the page scrolls/ });
    await userEvent.click(toggle);
    expect(draft().tracks['rune-line']).toBeUndefined();
    await userEvent.click(toggle);
    expect(draft().tracks['rune-line']!.keys.map(k => k.amount)).toEqual([0, 1]);
    await userEvent.click(screen.getByRole('button', { name: 'Edit Rune line in the Timeline' }));
    expect(useAppStore.getState().view).toBe('timeline');
  });

  it('removes the path and picks the next', async () => {
    await panel();
    await userEvent.click(screen.getByRole('button', { name: 'Remove path' }));
    expect(draft().paths.map(p => p.id)).toEqual(['root-glow', 'spare-run']);
    expect(draft().tracks['rune-line']).toBeUndefined();
    expect(usePathsStore.getState().selection?.path).toBe('root-glow');
  });
});
