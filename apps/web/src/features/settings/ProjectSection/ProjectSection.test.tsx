import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ProjectSection } from '.';

afterEach(() => clearMocks());

describe('the project settings', () => {
  it('renames the open project on Enter', async () => {
    await openMockWorkspace();
    render(<ProjectSection active />);
    const section = screen.getByRole('region', { name: /Project/ });
    expect(section).toHaveTextContent('Moon Island only');
    const name = within(section).getByRole('textbox', { name: 'Name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Moon Island at dusk{Enter}');
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.project?.name).toBe('Moon Island at dusk')
    );
  });

  it('shows nothing without an open project', async () => {
    await openMockWorkspace();
    const workspace = useAppStore.getState().workspace!;
    useAppStore.setState({ workspace: { ...workspace, project: null } });
    const { container } = render(<ProjectSection active={false} />);
    expect(container).toBeEmptyDOMElement();
  });
});
