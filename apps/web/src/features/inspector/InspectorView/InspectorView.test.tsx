import { render, screen } from '@testing-library/react';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import InspectorView from './InspectorView';

vi.mock('@/features/inspector/CompareCanvas', () => ({ default: () => null }));
vi.mock('@/features/inspector/LayoutCanvas', () => ({ default: () => null }));

afterEach(() => {
  clearMocks();
  useInspectorStore.setState({
    view: null,
    project: null,
    error: null,
    tab: 'compare',
    stop: null,
  });
  useLayoutStore.setState({ view: null, project: null });
});

describe('the Inspector view', () => {
  it('asks for a project first', () => {
    useAppStore.setState({ workspace: null });
    render(<InspectorView />);
    expect(screen.getByText('No project yet')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Inspector' })).toBeInTheDocument();
  });

  it("loads the project's models and compares the scene versions", async () => {
    await openMockWorkspace();
    render(<InspectorView />);
    expect(await screen.findByRole('heading', { name: 'Scene comparison' })).toBeInTheDocument();
    expect(screen.getByRole('complementary', { name: 'Model details' })).toBeInTheDocument();
  });

  it('offers to try again when the models cannot be read', async () => {
    const handle = await openMockWorkspace();
    mockIPC((cmd, args) =>
      cmd === 'inspect_project'
        ? Promise.reject(new Error('The export folder is gone.'))
        : handle(cmd, (args ?? {}) as Record<string, unknown>)
    );
    render(<InspectorView />);
    expect(await screen.findByText('The models could not be read')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('lays out the scene on the layout tab, with Save in the top bar', async () => {
    await openMockWorkspace();
    useInspectorStore.setState({ tab: 'layout' });
    render(<InspectorView />);
    expect(await screen.findByRole('heading', { name: 'Scene layout' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save layout' })).toBeDisabled();
  });
});
