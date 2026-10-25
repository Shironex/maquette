import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { CurrentView } from '.';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

describe('the current view', () => {
  it('shows Settings when the rail selects it', async () => {
    await openMockWorkspace();
    render(<CurrentView view="settings" />);
    expect(screen.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();
    expect(screen.getByRole('navigation', { name: 'Settings sections' })).toBeVisible();
  });

  it('shows the Lab top bar for the Lab', async () => {
    await openMockWorkspace();
    render(<CurrentView view="lab" />);
    expect(await screen.findByRole('button', { name: 'Run pipeline' })).toBeVisible();
  });
});
