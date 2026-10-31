import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import Inspector from '../Inspector/Inspector';

vi.mock('../CompareCanvas', () => ({ default: () => null }));

beforeEach(async () => {
  await openMockParts('compare');
  await useInspectorStore.getState().load('frostveil');
});
afterEach(() => clearMocks());

async function openVersions() {
  render(<Inspector view={useInspectorStore.getState().view!} />);
  await userEvent.click(screen.getByRole('button', { name: 'Provider vs provider' }));
  return screen.findByRole('table', { name: 'Version numbers' });
}

describe('version compare', () => {
  it('moves from the scene to an asset and shows the numbers with their dots', async () => {
    const table = await openVersions();
    expect(useInspectorStore.getState().comparison).toBe('asset/frost-lantern');
    const row = (label: string | RegExp) =>
      within(table).getByRole('rowheader', { name: label }).closest('tr')!;
    await waitFor(() => expect(row('Triangles')).toHaveTextContent('18.2k'));
    expect(row('Triangles')).toHaveTextContent('9.6k');
    expect(within(row('Credits')).getByRole('img', { name: 'better' })).toBeInTheDocument();
    expect(row('GPU memory')).toHaveTextContent('5.1 MB');
    expect(within(table).getAllByRole('columnheader')[1]).toHaveTextContent('Tripo v2');
    expect(screen.getByText(/nothing is deleted and no credits come back/)).toBeInTheDocument();
  });

  it('judges the texture at the stop chosen, and close up', async () => {
    const table = await openVersions();
    const aside = screen.getByRole('complementary', { name: 'Versions' });
    await within(aside).findByText('where it is biggest');
    await userEvent.click(within(aside).getByRole('button', { name: /Turntable, close up/ }));
    expect(
      within(table).getByRole('rowheader', { name: 'Texture detail, close up' })
    ).toBeInTheDocument();
  });

  it('keeps a version: it becomes current and both stay', async () => {
    await openVersions();
    await userEvent.click(await screen.findByRole('button', { name: 'Keep Tripo v2' }));
    await waitFor(() => {
      const lantern = useAppStore
        .getState()
        .workspace?.project?.assets?.find(a => a.id === 'frost-lantern');
      expect(lantern?.versions.at(-1)?.version).toBe(2);
      expect(lantern?.versions).toHaveLength(3);
    });
  });
});
