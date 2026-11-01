import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTrashStore } from '@/state/use-trash-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import BoardHeader from './BoardHeader';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({
    project: null,
    jobs: [],
    queueOpen: false,
    checklistOpen: false,
    busy: [],
  });
  useTrashStore.setState({ project: null, entries: [], restoring: null });
  useAppStore.setState({ notice: null });
  window.localStorage.clear();
  forgetStoredPrefs();
  useBoardPrefsStore.setState({ byProject: {} });
});

describe('BoardHeader', () => {
  it('says the last run shipped nothing, and nothing when it did', async () => {
    await usePipelineStore.getState().load('frostveil');
    const { rerender } = render(<BoardHeader />);
    expect(screen.queryByText('Not shipped')).toBeNull();
    const view = usePipelineStore.getState().view!;
    const last = { ...view.runs[0]!, state: 'done' as const, notShipped: 'No site repo.' };
    usePipelineStore.setState({ view: { ...view, active: null, runs: [last] } });
    rerender(<BoardHeader />);
    expect(await screen.findByText('Not shipped')).toBeVisible();
    usePipelineStore.setState({ view: null });
  });

  it('gives every button of the header a tooltip', async () => {
    await openMockGame('crypt');
    render(<BoardHeader />);
    for (const button of screen.getAllByRole('button')) {
      await userEvent.hover(button);
      expect(await screen.findByRole('tooltip'), button.textContent ?? '').toBeVisible();
      await userEvent.unhover(button);
      await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull());
    }
  });

  it('names the project and keeps the old buttons out of the row', () => {
    render(<BoardHeader />);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Frostveil');
    for (const gone of [/^By hand/, /^Export pack/, /^Built asset/, /^Removed/])
      expect(screen.queryByRole('button', { name: gone })).toBeNull();
  });

  it('switches the Board to the list and remembers it for the project', async () => {
    render(<BoardHeader />);
    const layout = screen.getByRole('group', { name: 'Board layout' });
    const list = within(layout).getByRole('button', { name: 'List' });
    expect(within(layout).getByRole('button', { name: 'Board' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(list);
    expect(list).toHaveAttribute('aria-pressed', 'true');
    expect(window.localStorage.getItem('maquette.board.frostveil')).toContain('"list"');
  });
});
