import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands, type InspectView } from '@/ipc/client';
import { pickSides } from '@/state/inspector';
import { useInspectorStore } from '@/state/use-inspector-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import CompareMenu from './CompareMenu';

let view: InspectView;

beforeEach(async () => {
  await openMockWorkspace();
  view = await commands.inspectProject('frostveil');
});
afterEach(() => {
  clearMocks();
  useInspectorStore.setState({ comparison: null, left: null, right: null });
});

describe('the compare menu', () => {
  it('names both versions and lists the comparisons, the current one off', async () => {
    render(<CompareMenu view={view} pick={pickSides(view, null, null, null)!} />);
    const button = screen.getByRole('button', { name: /v6\s*vs\s*v7/ });
    expect(button).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(button);
    expect(button).toHaveAttribute('aria-expanded', 'true');
    const scene = screen.getByRole('option', { name: 'Scene' });
    expect(scene).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('combobox', { name: 'Search compare' })).toHaveFocus();
  });

  it('offers the other versions against the current one, and picks one', async () => {
    const pick = pickSides(view, 'scene', 'scene/v7', 'scene/v7')!;
    render(<CompareMenu view={view} pick={{ ...pick, left: null }} />);
    await userEvent.click(screen.getByRole('button'));
    await userEvent.click(screen.getByRole('option', { name: 'Compare v6 with v7' }));
    expect(useInspectorStore.getState().left).toBe('scene/v6');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('filters as you type, picks the first match on Enter and clears before closing', async () => {
    const pick = pickSides(view, 'scene', 'scene/v7', 'scene/v7')!;
    render(<CompareMenu view={view} pick={{ ...pick, left: null }} />);
    await userEvent.click(screen.getByRole('button'));
    const search = screen.getByRole('combobox', { name: 'Search compare' });
    await userEvent.type(search, 'zzz');
    expect(screen.getByText(/No model matches/)).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(search).toHaveValue('');
    expect(within(screen.getByRole('listbox')).getAllByRole('option').length).toBeGreaterThan(0);
    await userEvent.type(search, 'v6{Enter}');
    expect(useInspectorStore.getState().left).toBe('scene/v6');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('closes on Escape once the search is empty', async () => {
    render(<CompareMenu view={view} pick={pickSides(view, null, null, null)!} />);
    await userEvent.click(screen.getByRole('button'));
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('listbox')).toBeNull();
  });
});
