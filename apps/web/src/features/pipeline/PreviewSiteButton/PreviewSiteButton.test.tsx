import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PreviewSiteButton } from '.';

afterEach(() => clearMocks());

describe('Preview on site', () => {
  it('opens the site and says where', async () => {
    await openMockWorkspace();
    render(<PreviewSiteButton />);
    await userEvent.click(screen.getByRole('button', { name: 'Preview on site' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice).toMatchObject({
        status: 'done',
        title: 'Opened the site preview',
        detail: 'http://localhost:15292/',
      })
    );
  });

  it('waits for a site repo and says where to pick one', async () => {
    await openMockWorkspace();
    await useProjectsStore.getState().switchTo('rooftop-cinema');
    render(<PreviewSiteButton />);
    const button = screen.getByRole('button', { name: 'Preview on site' });
    expect(button).toHaveAttribute('aria-disabled', 'true');
    await userEvent.hover(button);
    expect(await screen.findByText('Pick the site repo in Settings > Project first.')).toBeTruthy();
  });
});
