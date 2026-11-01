import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useFirstRunTips } from '@/state/use-first-run-tips';
import { useInboxStore } from '@/state/use-inbox-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';
import { useSettingsTarget } from '@/state/use-settings-target';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { NextStepStrip } from '.';

const KEY = 'maquette.board.nextStepDismissed.frostveil';
const openPanel = useProvidersStore.getState().openPanel;
const initialTools = useAppStore.getState().tools;

afterEach(() => {
  clearMocks();
  window.localStorage.removeItem(KEY);
  useFirstRunTips.setState({ hidden: false });
  window.localStorage.removeItem('maquette.board.nextStepDismissed.other');
  useInboxStore.setState({ snapshot: null });
  usePipelineStore.setState({ view: null });
  useProvidersStore.setState({ openPanel, jobs: [] });
  useSettingsTarget.setState({ asked: null });
  useAppStore.setState({ view: 'board', tools: initialTools });
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
        thumbnail: stage === 'concept' ? 'concepts/a/a_01.png' : 'refs/a.png',
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

  const dismiss = () => userEvent.click(screen.getByRole('button', { name: 'Dismiss this step' }));
  const inbox = (n: number) =>
    act(() =>
      useInboxStore.setState({
        snapshot: { items: Array.from({ length: n }, (_, id) => ({ id })) } as never,
      })
    );

  it('remembers a dismissed step across a remount, and brings it back when it changes', async () => {
    await withAssets(['refs']);
    inbox(2);
    const { unmount } = render(<NextStepStrip />);
    await dismiss();
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent('A0 has refs');
    await dismiss();
    expect(screen.queryByRole('region')).toBeNull();
    unmount();
    render(<NextStepStrip />);
    expect(screen.queryByRole('region')).toBeNull();
    inbox(3);
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent(
      '3 models wait in the Inbox'
    );
  });

  it('keeps a dismissal to its own project', async () => {
    await withAssets(['refs']);
    render(<NextStepStrip />);
    await dismiss();
    expect(screen.queryByRole('region')).toBeNull();
    const workspace = useAppStore.getState().workspace!;
    const other = { ...workspace.project!, id: 'other' };
    act(() => useAppStore.setState({ workspace: { ...workspace, project: other } }));
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent('A0 has refs');
  });

  it('forgets a dismissal once a settled check says the step is gone', async () => {
    await withAssets(['refs']);
    const tools = useAppStore.getState().tools;
    const blender = (status: string) =>
      act(() =>
        useAppStore.setState({
          tools: {
            ...tools,
            checking: false,
            checkedAt: 1,
            reports: [{ id: 'blender', status, required: true } as never],
          },
        })
      );
    usePipelineStore.setState({ view: { project: 'frostveil', runs: [] } as never });
    inbox(0);
    blender('missing');
    render(<NextStepStrip />);
    await dismiss();
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent('A0 has refs');
    blender('found');
    blender('missing');
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent('Blender');
  });

  it('holds the dismissal while a check is running', async () => {
    await withAssets(['refs']);
    const tools = useAppStore.getState().tools;
    const missing = [{ id: 'blender', status: 'missing', required: true } as never];
    usePipelineStore.setState({ view: { project: 'frostveil', runs: [] } as never });
    inbox(0);
    act(() =>
      useAppStore.setState({ tools: { ...tools, checking: false, checkedAt: 1, reports: missing } })
    );
    render(<NextStepStrip />);
    await dismiss();
    act(() =>
      useAppStore.setState({ tools: { ...tools, checking: true, checkedAt: 1, reports: missing } })
    );
    act(() =>
      useAppStore.setState({ tools: { ...tools, checking: false, checkedAt: 2, reports: missing } })
    );
    expect(screen.getByRole('region', { name: 'Next step' })).toHaveTextContent('A0 has refs');
  });

  it('opens the refs of a card that has none', async () => {
    await withAssets(['refs']);
    const project = useAppStore.getState().workspace!.project!;
    act(() =>
      useAppStore.getState().setProject({
        ...project,
        assets: project.assets!.map(a => ({ ...a, thumbnail: null, refs: [] })),
      })
    );
    const open = vi.fn(async () => undefined);
    const before = useRefsStore.getState().open;
    useRefsStore.setState({ open });
    render(<NextStepStrip />);
    await userEvent.click(screen.getByRole('button', { name: 'Add refs' }));
    expect(open).toHaveBeenCalledWith('frostveil', 'a0');
    useRefsStore.setState({ open: before });
  });

  it('looks at Blender again when the window comes back while it has trouble', async () => {
    await withAssets(['refs']);
    const tools = useAppStore.getState().tools;
    const recheck = vi.fn(async () => undefined);
    const before = useAppStore.getState().recheckToolsOnFocus;
    act(() =>
      useAppStore.setState({
        recheckToolsOnFocus: recheck,
        tools: { ...tools, reports: [{ id: 'blender', status: 'missing' } as never] },
      })
    );
    render(<NextStepStrip />);
    act(() => void window.dispatchEvent(new Event('focus')));
    expect(recheck).toHaveBeenCalledOnce();
    useAppStore.setState({ recheckToolsOnFocus: before });
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
