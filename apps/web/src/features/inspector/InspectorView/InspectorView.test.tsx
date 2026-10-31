import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import InspectorView from './InspectorView';

vi.mock('@/features/inspector/CompareCanvas', () => ({ default: () => null }));
vi.mock('@/features/inspector/LayoutCanvas', () => ({ default: () => null }));
vi.mock('@/features/inspector/PathsCanvas', () => ({ default: () => null }));
vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

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
    expect(screen.getByRole('button', { name: 'Save layout' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('draws the paths through the scene on the paths tab', async () => {
    await openMockWorkspace();
    useInspectorStore.setState({ tab: 'paths' });
    render(<InspectorView />);
    expect(await screen.findByRole('heading', { name: 'Scene paths' })).toBeInTheDocument();
    expect(screen.getByRole('complementary', { name: 'Path details' })).toBeInTheDocument();
  });
});

describe('the Inspector tabs and the page under them', () => {
  const expectOnly = (name: string) => {
    const selected = screen
      .getAllByRole('tab')
      .filter(t => t.getAttribute('aria-selected') === 'true');
    expect(selected).toEqual([screen.getByRole('tab', { name })]);
  };
  const compare = () => screen.findByRole('complementary', { name: 'Model details' });
  const away = async (page: () => Promise<unknown>) => {
    await page();
    expect(screen.queryByRole('complementary', { name: 'Model details' })).toBeNull();
  };
  const budget = () => away(() => screen.findAllByText(/^Over budget/));
  const layout = () => away(() => screen.findByRole('heading', { name: 'Scene layout' }));
  const paths = () => away(() => screen.findByRole('heading', { name: 'Scene paths' }));
  const parts = () => away(() => screen.findByRole('combobox', { name: 'Selected model' }));
  const renderPage = () =>
    away(() => screen.findByRole('heading', { name: /^(Styles|Nothing to render yet)$/ }));

  /** Each tab, then back the other way, so a tab that sticks or falls through cannot pass. */
  async function clickEvery(pages: readonly (readonly [string, () => Promise<unknown>])[]) {
    expect(screen.getAllByRole('tab').map(t => t.textContent)).toEqual(pages.map(([n]) => n));
    for (const [name, page] of [...pages, ...[...pages].reverse()]) {
      await userEvent.click(screen.getByRole('tab', { name }));
      expectOnly(name);
      await page();
    }
  }

  it('shows its own page for each tab of a game project', async () => {
    await openMockGame('crypt');
    render(<InspectorView />);
    await compare();
    await clickEvery([
      ['Compare', compare],
      ['Budget', budget],
      ['Parts', parts],
      ['Render', renderPage],
    ]);
  });

  it('shows its own page for each tab of a website project', async () => {
    await openMockWorkspace();
    render(<InspectorView />);
    await compare();
    await clickEvery([
      ['Compare', compare],
      ['Layout', layout],
      ['Paths', paths],
      ['Parts', parts],
      ['Render', renderPage],
    ]);
  });

  it('falls back to Compare, in the tabs and the page, for a tab the project lacks', async () => {
    await openMockGame('crypt');
    useInspectorStore.setState({ tab: 'layout' });
    render(<InspectorView />);
    await compare();
    expectOnly('Compare');
  });
});
