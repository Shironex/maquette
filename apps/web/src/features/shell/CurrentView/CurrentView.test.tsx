import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { CurrentView } from '.';
import { ViewBoundary } from './ViewBoundary';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

describe('the current view', () => {
  it('shows Settings when the rail selects it', async () => {
    await openMockWorkspace();
    render(<CurrentView view="settings" />);
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Settings' }, { timeout: 4000 })
    ).toBeVisible();
    expect(screen.getByRole('navigation', { name: 'Settings sections' })).toBeVisible();
  });

  it('shows the Lab top bar for the Lab', async () => {
    await openMockWorkspace();
    render(<CurrentView view="lab" />);
    expect(await screen.findByRole('button', { name: 'Run pipeline' })).toBeVisible();
  });

  it('shows a busy view while a view loads, and the Board at once', async () => {
    await openMockWorkspace();
    const { container, rerender } = render(<CurrentView view="library" />);
    expect(container.querySelector('main[aria-busy="true"]')).not.toBeNull();
    expect(
      await screen.findByRole('searchbox', { name: 'Find a model' }, { timeout: 4000 })
    ).toBeVisible();
    rerender(<CurrentView view="board" />);
    expect(container.querySelector('main[aria-busy="true"]')).toBeNull();
  });

  it('says so when a view fails to open', async () => {
    await openMockWorkspace();
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    usePipelineStore.setState({ view: null });
    const Broken = () => {
      throw new Error('The chunk did not load.');
    };
    render(
      <ViewBoundary view="lab">
        <Broken />
      </ViewBoundary>
    );
    expect(screen.getByText('This view could not open')).toBeVisible();
    expect(screen.getByText('The chunk did not load.')).toBeVisible();
    error.mockRestore();
  });
});
