import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { LabView } from '.';

const lab = useLabStore.getState();
const app = useAppStore.getState();

afterEach(() => {
  cleanup();
  clearMocks();
  useLabStore.setState(lab, true);
  useAppStore.setState(app, true);
});

describe('the Lab view', () => {
  it('asks for a project when none is open', () => {
    render(<LabView />);
    expect(screen.getByText('No project open')).toBeVisible();
  });

  it('offers Try again when the concepts could not be read, and it reads them', async () => {
    await openMockWorkspace();
    useLabStore.setState({ view: null, loadFailed: true });
    render(<LabView />);
    expect(screen.getByText('The concepts could not be read')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('region', { name: 'New direction' })).toBeVisible();
  });

  it('shows the composer, the results and the queue once the concepts load', async () => {
    await openMockWorkspace();
    render(<LabView />);
    expect(screen.getByText('Reading the concepts...')).toBeVisible();
    await useLabStore.getState().load('frostveil');
    expect(await screen.findByRole('region', { name: 'New direction' })).toBeVisible();
    expect(screen.getByRole('complementary', { name: 'Image queue' })).toBeVisible();
  });
});
