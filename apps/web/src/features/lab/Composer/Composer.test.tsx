import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView, ToolReport } from '@/ipc/client';
import { labDirections, labRefs } from '@/ipc/mock/lab-fixtures';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { Composer } from '.';

const view: LabView = {
  project: 'moon-island',
  projectDir: '/w/moon-island',
  directions: labDirections(false),
  jobs: [],
  refs: labRefs(),
  concurrency: 4,
};
const lab = useLabStore.getState();
const app = useAppStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
  useAppStore.setState(app, true);
});

describe('the composer', () => {
  it('generates once the direction has a name and a prompt', async () => {
    const generate = vi.fn(async () => {});
    useLabStore.setState({ generate });
    render(<Composer view={{ ...view, directions: [] }} />);
    const button = screen.getByRole('button', { name: 'Generate 4 concepts' });
    expect(button).toHaveAttribute('aria-disabled', 'true');
    await userEvent.type(screen.getByLabelText('Direction name'), 'Harbour');
    await userEvent.type(screen.getByLabelText('Prompt'), 'A small harbour at dawn');
    expect(useLabStore.getState().draft).toMatchObject({ name: 'Harbour' });
    await userEvent.click(button);
    expect(generate).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Runs through Codex CLI, 4 at a time')).toBeVisible();
  });

  it('adds to an earlier direction with the same name', () => {
    act(() => useLabStore.getState().setDraft({ name: 'Late train' }));
    render(<Composer view={view} />);
    expect(screen.getByRole('heading', { name: 'Add to direction' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'New' })).toBeVisible();
    expect(screen.getByText('Earlier directions')).toBeVisible();
  });

  it('says when Codex CLI is not ready', () => {
    const codex = { id: 'codex', status: 'missing', hint: 'Run codex login.' } as ToolReport;
    useAppStore.setState({ tools: { ...app.tools, reports: [codex] } });
    render(<Composer view={view} />);
    expect(screen.getByText('Codex CLI is not ready. Run codex login.')).toBeVisible();
  });
});
