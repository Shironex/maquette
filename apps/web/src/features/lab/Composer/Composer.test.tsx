import { clock } from '@/lib/clock';
import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView, ToolReport } from '@/ipc/client';
import { labDirections, labRefs } from '@/ipc/mock/lab-fixtures';
import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';
import { Composer } from '.';
import { EMPTY_STYLE } from '@/state/lab-style';
import { frostveilStyleBoard } from '@/ipc/mock/style-board';
import { mockUsage } from '@/ipc/mock/create-codex-usage-handlers';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';

const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: labDirections(false),
  jobs: [],
  refs: labRefs(),
  concurrency: 4,
  style: EMPTY_STYLE,
};
const lab = useLabStore.getState();
const app = useAppStore.getState();
const usage = useCodexUsageStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
  useAppStore.setState(app, true);
  useCodexUsageStore.setState(usage, true);
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
    act(() => useLabStore.getState().setDraft({ name: 'Veil gate kit' }));
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

  it('carries the project style unless the switch turns it off', async () => {
    render(<Composer view={{ ...view, style: frostveilStyleBoard() }} />);
    const toggle = screen.getByRole('switch', { name: /^Use the project style/ });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    expect(useLabStore.getState().draft.useStyle).toBe(false);
    expect(screen.getByText('This direction keeps its own look')).toBeVisible();
  });

  it('offers to set up a style board while there is none', () => {
    render(<Composer view={view} />);
    expect(screen.getByRole('button', { name: 'Set up' })).toBeVisible();
    expect(screen.queryByRole('switch', { name: /^Use the project style/ })).toBeNull();
  });

  it('warns before a large round when little Codex usage is left', () => {
    useCodexUsageStore.setState({ usage: mockUsage('low', clock.nowMs()) });
    render(<Composer view={view} />);
    expect(screen.getByRole('note')).toHaveTextContent(
      /^Only 14% of Codex's 5h usage is left, which resets (\w{3} )?\d\d:\d\d\. A round of 4 may stop at the usage limit and wait\.$/
    );
    act(() => useLabStore.getState().setDraft({ count: 2 }));
    expect(screen.queryByRole('note')).toBeNull();
  });
});
