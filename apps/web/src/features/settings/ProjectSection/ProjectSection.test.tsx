import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ProjectSection } from '.';

afterEach(() => clearMocks());

describe('the project settings', () => {
  it('renames the open project on Enter', async () => {
    await openMockWorkspace();
    render(<ProjectSection active />);
    const section = screen.getByRole('region', { name: /Project/ });
    expect(section).toHaveTextContent('Frostveil only');
    const name = within(section).getByRole('textbox', { name: 'Name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Frostveil at dusk{Enter}');
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.project?.name).toBe('Frostveil at dusk')
    );
  });

  it('shows nothing without an open project', async () => {
    await openMockWorkspace();
    const workspace = useAppStore.getState().workspace!;
    useAppStore.setState({ workspace: { ...workspace, project: null } });
    const { container } = render(<ProjectSection active={false} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('saves the dev command for the Page view', async () => {
    await openMockWorkspace();
    render(<ProjectSection active />);
    const field = screen.getByRole('textbox', { name: 'Site dev command' });
    await waitFor(() => expect(field).toBeEnabled());
    expect(field).toHaveAttribute('placeholder', 'pnpm run dev --port {port}');
    await userEvent.type(field, 'pnpm exec astro dev --port {{port}{Enter}');
    await waitFor(async () =>
      expect((await commands.pageGet('frostveil')).settings.devCommand).toBe(
        'pnpm exec astro dev --port {port}'
      )
    );
  });
});
