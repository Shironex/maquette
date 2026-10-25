import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { NewProjectSheet } from '.';
import { tooltipText } from '@/test/tooltipText';

const initial = useProjectsStore.getState();

afterEach(() => {
  cleanup();
  clearMocks();
  useProjectsStore.setState(initial, true);
});

describe('the new project sheet', () => {
  it('names the open project and waits for a name', async () => {
    await openMockWorkspace();
    render(<NewProjectSheet initialMode="template" />);
    expect(
      screen.getByText('It opens when it is ready. Moon Island stays as it is.')
    ).toBeVisible();
    const create = screen.getByRole('button', { name: 'Create project' });
    expect(create).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(create)).toBe('Name it first');
    expect(screen.getByLabelText('Project name')).toHaveFocus();
  });

  it('creates the project and reports the close', async () => {
    await openMockWorkspace();
    const create = vi.fn(async () => true);
    useProjectsStore.setState({ create });
    const onClosed = vi.fn();
    render(<NewProjectSheet initialMode="template" onClosed={onClosed} />);
    await userEvent.type(screen.getByLabelText('Project name'), 'Harbour');
    await userEvent.click(screen.getByRole('button', { name: 'Create project' }));
    expect(create).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'Harbour', mode: 'template' })
    );
    expect(onClosed).toHaveBeenCalledTimes(1);
  });

  it('submits on Cmd+Enter', async () => {
    await openMockWorkspace();
    const create = vi.fn(async () => false);
    useProjectsStore.setState({ create });
    render(<NewProjectSheet initialMode="template" />);
    const name = screen.getByLabelText('Project name');
    await userEvent.type(name, 'Harbour');
    fireEvent.keyDown(name, { key: 'Enter', metaKey: true });
    expect(create).toHaveBeenCalledTimes(1);
  });

  it('closes on Escape and on a click outside the form', async () => {
    await openMockWorkspace();
    const closeDialog = vi.fn();
    useProjectsStore.setState({ closeDialog });
    const onClosed = vi.fn();
    const { container } = render(<NewProjectSheet initialMode="import" onClosed={onClosed} />);
    await userEvent.keyboard('{Escape}');
    expect(closeDialog).toHaveBeenCalledTimes(1);
    await userEvent.click(container.firstElementChild as HTMLElement);
    expect(closeDialog).toHaveBeenCalledTimes(2);
    expect(onClosed).toHaveBeenCalledTimes(2);
  });

  it('focuses the first field and asks for a folder when importing', async () => {
    await openMockWorkspace();
    render(<NewProjectSheet initialMode="import" />);
    expect(screen.getByLabelText('Folder to import')).toHaveFocus();
    expect(await tooltipText(screen.getByRole('button', { name: 'Import project' }))).toBe(
      'Name it and pick a folder first'
    );
  });
});
