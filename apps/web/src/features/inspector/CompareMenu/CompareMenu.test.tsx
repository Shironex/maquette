import { render, screen } from '@testing-library/react';
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
  view = await commands.inspectProject('moon-island');
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
    const scene = screen.getByRole('menuitem', { name: 'Scene' });
    expect(scene).toBeDisabled();
  });

  it('offers the other versions against the current one, and picks one', async () => {
    const pick = pickSides(view, 'scene', 'scene/v7', 'scene/v7')!;
    render(<CompareMenu view={view} pick={{ ...pick, left: null }} />);
    await userEvent.click(screen.getByRole('button'));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Compare v6 with v7' }));
    expect(useInspectorStore.getState().left).toBe('scene/v6');
  });
});
