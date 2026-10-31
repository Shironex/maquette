import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { type JobState, type Provider } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { forgetStoredPrefs, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTrashStore } from '@/state/use-trash-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import SendsButton from './SendsButton';

const job = (id: string, provider: Provider, state: JobState) =>
  ({ id, provider, state, assetName: id, batch: null }) as never;

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

describe('Sends', () => {
  const sends = () => screen.queryByRole('button', { name: /^Sends/ });

  it('is hidden with nothing in flight, finished or failed sends included', () => {
    useProvidersStore.setState({ jobs: [job('a', 'tripo', 'failed'), job('b', 'meshy', 'inbox')] });
    render(<SendsButton />);
    expect(sends()).toBeNull();
  });

  it('counts a By hand send once, not once per list', () => {
    useProvidersStore.setState({
      jobs: [job('a', 'by-hand', 'waiting'), job('b', 'by-hand', 'waiting')],
    });
    render(<SendsButton />);
    expect(sends()).toHaveTextContent(/^Sends2$/);
  });

  it('goes straight to By hand when only By hand sends are under way', async () => {
    useProvidersStore.setState({ jobs: [job('a', 'by-hand', 'waiting')] });
    render(<SendsButton />);
    await userEvent.click(sends()!);
    expect(useProvidersStore.getState().checklistOpen).toBe(true);
    expect(useProvidersStore.getState().queueOpen).toBe(false);
  });

  it('goes to the 3D queue when paid sends are under way', async () => {
    useProvidersStore.setState({
      jobs: [job('a', 'by-hand', 'waiting'), job('b', 'tripo', 'running')],
    });
    render(<SendsButton />);
    expect(sends()).toHaveTextContent(/^Sends2$/);
    await userEvent.click(sends()!);
    expect(useProvidersStore.getState().queueOpen).toBe(true);
    expect(useProvidersStore.getState().checklistOpen).toBe(false);
  });

  it('marks a send that waits on you, and one that failed', async () => {
    useProvidersStore.setState({
      jobs: [job('a', 'meshy', 'approval'), job('b', 'tripo', 'failed')],
    });
    render(<SendsButton />);
    const button = sends()!;
    expect(button).toHaveAccessibleName(/1 failed/);
    expect(within(button).getByText('1')).toHaveClass('text-clay-text');
    await userEvent.hover(button);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      '1 send in flight: 1 waiting for approval, 1 failed'
    );
  });

  it('stays amber while the providers do the waiting', () => {
    useProvidersStore.setState({ jobs: [job('a', 'tripo', 'running')] });
    render(<SendsButton />);
    expect(within(sends()!).getByText('1')).toHaveClass('text-amber');
  });
});
