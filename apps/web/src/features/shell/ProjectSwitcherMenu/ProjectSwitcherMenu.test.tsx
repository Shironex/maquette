import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createRef } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { isSwitcherShortcut, switchNote } from '@/state/projects';
import { useAppStore } from '@/state/use-app-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { NewProjectDialog } from '@/features/project/NewProjectDialog';
import { ProjectSwitcherMenu } from '.';

function renderMenu() {
  const anchor = createRef<HTMLButtonElement>();
  render(
    <>
      <button ref={anchor} type="button">
        Switcher
      </button>
      <ProjectSwitcherMenu anchorRef={anchor} collapsed={false} shortcut="⌘P" />
      <NewProjectDialog />
    </>
  );
  return anchor;
}

afterEach(() => {
  clearMocks();
  useProjectsStore.setState({ list: [], menuOpen: false, dialog: null });
});

describe('the project switcher', () => {
  it('lists the projects, marks the open one and switches with the keyboard', async () => {
    await openMockWorkspace();
    const anchor = renderMenu();
    useProjectsStore.getState().openMenu();
    const moon = await screen.findByRole('menuitemradio', { name: /Moon Island/ });
    expect(moon).toHaveAttribute('aria-checked', 'true');
    await waitFor(() =>
      expect(screen.getByText(/Character showcase · 1 asset/)).toBeInTheDocument()
    );
    expect(moon).toHaveFocus();

    await userEvent.keyboard('{ArrowDown}{Enter}');
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.project?.id).toBe('rooftop-cinema')
    );
    expect(useAppStore.getState().workspace?.config.currentProject).toBe('rooftop-cinema');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(anchor.current).toHaveFocus();
  });

  it('closes on Escape and gives focus back to the switcher', async () => {
    await openMockWorkspace();
    const anchor = renderMenu();
    useProjectsStore.getState().openMenu();
    await screen.findByRole('menu', { name: 'Projects' });
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(anchor.current).toHaveFocus();
    expect(useAppStore.getState().workspace?.project?.id).toBe('moon-island');
  });

  it('searches once there are more than six projects', async () => {
    await openMockWorkspace('configured', 'many');
    renderMenu();
    useProjectsStore.getState().openMenu();
    const search = await screen.findByRole('searchbox', { name: 'Find a project' });
    await waitFor(() => expect(screen.getAllByRole('menuitemradio')).toHaveLength(8));
    await userEvent.type(search, 'harbour{Enter}');
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.project?.id).toBe('harbour-lights')
    );
  });

  it('creates a project from the compact form and opens it', async () => {
    await openMockWorkspace();
    renderMenu();
    useProjectsStore.getState().openMenu();
    await userEvent.click(await screen.findByRole('menuitem', { name: 'New project' }));
    const dialog = screen.getByRole('dialog', { name: 'New project' });
    expect(dialog).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Room/ }));
    await userEvent.type(screen.getByLabelText('Project name'), 'Maquette landing');
    await userEvent.click(screen.getByRole('button', { name: 'Create project' }));
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.project?.name).toBe('Maquette landing')
    );
    expect(useAppStore.getState().workspace?.project?.template).toBe('room');
    expect(useAppStore.getState().workspace?.projects).toHaveLength(4);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('opens Import folder on the import form', async () => {
    await openMockWorkspace();
    renderMenu();
    useProjectsStore.getState().openMenu();
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Import folder' }));
    expect(screen.getByRole('dialog', { name: 'Import a folder' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Import project' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});

describe('switcher helpers', () => {
  it('opens on Cmd+P on macOS and Ctrl+P elsewhere', () => {
    const key = (k: Partial<KeyboardEvent>) => ({
      key: 'p',
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      ...k,
    });
    expect(isSwitcherShortcut(key({ metaKey: true }), true)).toBe(true);
    expect(isSwitcherShortcut(key({ ctrlKey: true }), true)).toBe(false);
    expect(isSwitcherShortcut(key({ ctrlKey: true, key: 'P' }), false)).toBe(true);
    expect(isSwitcherShortcut(key({ metaKey: true }), false)).toBe(false);
    expect(isSwitcherShortcut(key({ ctrlKey: true, shiftKey: true }), false)).toBe(false);
  });

  it('says in one line what keeps running', () => {
    const base = {
      id: 'moon-island',
      name: 'Moon Island',
      template: 'floating-diorama' as const,
      target: 'website' as const,
      siteRepo: null,
      siteRepoName: null,
      siteRepoState: 'none' as const,
      createdAt: '',
      assetCount: 0,
      dir: '',
      thumb: null,
    };
    expect(switchNote({ ...base, activity: { run: null, concepts: 0, sends: 0 } })).toBeNull();
    expect(
      switchNote({ ...base, activity: { run: 'moon-island-run-25', concepts: 0, sends: 0 } })
    ).toBe('Run 25 keeps going in Moon Island while you switch.');
    expect(
      switchNote({ ...base, activity: { run: 'moon-island-run-25', concepts: 2, sends: 1 } })
    ).toBe('Run 25, 2 images and 1 send keep going in Moon Island while you switch.');
  });
});
