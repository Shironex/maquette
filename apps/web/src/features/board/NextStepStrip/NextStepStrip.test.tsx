import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { writeJson } from '@/lib/storage';
import { useAppStore } from '@/state/use-app-store';
import { useFirstRunTips } from '@/state/use-first-run-tips';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useSettingsTarget } from '@/state/use-settings-target';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { NextStepStrip } from '.';

const KEY = 'maquette.board.nextStepDismissed';
const openPanel = useProvidersStore.getState().openPanel;

afterEach(() => {
  clearMocks();
  window.localStorage.removeItem(KEY);
  useFirstRunTips.setState({ hidden: false });
  useInboxStore.setState({ snapshot: null });
  useProvidersStore.setState({ openPanel, jobs: [] });
  useSettingsTarget.setState({ asked: null });
  useAppStore.setState({ view: 'board' });
  vi.restoreAllMocks();
});

async function withAssets(stages: readonly ('refs' | 'concept')[]) {
  await openMockWorkspace();
  const project = useAppStore.getState().workspace!.project!;
  const base = project.assets![0]!;
  act(() =>
    useAppStore.getState().setProject({
      ...project,
      assets: stages.map((stage, i) => ({
        ...base,
        id: `a${i}`,
        name: `A${i}`,
        stage,
        versions: [],
      })),
    })
  );
  useProvidersStore.setState({ jobs: [] });
  useInboxStore.setState({ snapshot: null });
}

describe('the Next step strip', () => {
  it('opens Send to 3D for the one asset at Refs', async () => {
    await withAssets(['refs']);
    const panel = vi.fn(async () => undefined);
    useProvidersStore.setState({ openPanel: panel });
    render(<NextStepStrip />);
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent('A0 has refs');
    await userEvent.click(screen.getByRole('button', { name: 'Send to 3D' }));
    expect(panel).toHaveBeenCalledWith('frostveil', 'a0');
  });

  it('remembers a dismissed step and shows a different one', async () => {
    await withAssets(['refs']);
    const { unmount } = render(<NextStepStrip />);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss this step' }));
    expect(screen.queryByRole('region')).toBeNull();
    unmount();
    render(<NextStepStrip />);
    expect(screen.queryByRole('region')).toBeNull();
    await withAssets(['concept']);
    expect(await screen.findByRole('region', { name: 'Next step' })).toHaveTextContent(
      'Pick the concepts'
    );
  });

  it('honours a remembered dismissal of something else', async () => {
    writeJson(KEY, 'blender');
    await withAssets(['concept']);
    render(<NextStepStrip />);
    await userEvent.click(screen.getByRole('button', { name: 'Open Concept lab' }));
    expect(useAppStore.getState().view).toBe('lab');
  });

  it('lands on Toolchain when Blender is missing', async () => {
    await withAssets(['concept']);
    const tools = useAppStore.getState().tools;
    act(() =>
      useAppStore.setState({
        tools: {
          ...tools,
          checking: false,
          reports: [{ id: 'blender', status: 'missing', required: true } as never],
        },
      })
    );
    render(<NextStepStrip />);
    await userEvent.click(screen.getByRole('button', { name: 'Open Toolchain' }));
    expect(useSettingsTarget.getState().asked).toBe('toolchain');
    expect(useAppStore.getState().view).toBe('settings');
  });
});
