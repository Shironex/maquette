import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { WorkspaceSection } from '.';

afterEach(() => {
  clearMocks();
  useAppStore.setState({ notice: null });
});

async function renderSection() {
  await openMockWorkspace();
  const config = useAppStore.getState().workspace!.config;
  render(<WorkspaceSection active={false} config={config} />);
  return { config, section: screen.getByRole('region', { name: 'Workspace' }) };
}

describe('the workspace settings', () => {
  it('lists the workspace folders and clears the cache', async () => {
    const { config, section } = await renderSection();
    expect(within(section).getByText(`${config.layout.inbox}/`)).toBeVisible();
    expect(within(section).getByText('Tripo models land here')).toBeVisible();
    const clear = within(section).getByRole('button', { name: 'Clear cache' });
    await waitFor(() => expect(clear).toBeEnabled());
    await userEvent.click(clear);
    await waitFor(() => expect(useAppStore.getState().notice?.title).toBe('Cache cleared'));
    expect(within(section).getByText('0 B')).toBeVisible();
    expect(clear).toHaveAttribute('aria-disabled', 'true');
  });

  it('says so when the picked folder is not a workspace', async () => {
    const { section } = await renderSection();
    const field = within(section).getByRole('textbox', { name: 'Root folder' });
    await userEvent.clear(field);
    await userEvent.type(field, '~/Sketches{Enter}');
    expect(
      await within(section).findByText(/Pick a folder that holds maquette.json/)
    ).toBeVisible();
  });
});
