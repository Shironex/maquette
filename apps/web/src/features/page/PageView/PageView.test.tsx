import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PageView } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

async function show(page?: string) {
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    page
  );
  render(<PageView />);
}

describe('the page view', () => {
  it('starts the site dev server and shows the page once it runs', async () => {
    await show();
    expect(await screen.findByText("Starting the site's dev server")).toBeVisible();
    expect(
      await screen.findByTitle('The frostveil-site page from the site dev page', undefined, {
        timeout: 4000,
      })
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open in browser' })).not.toHaveAttribute(
      'aria-disabled'
    );
    expect(screen.getByRole('region', { name: 'Scroll scrubber' })).toBeVisible();
  });

  it('says when the dev server did not start', async () => {
    await show('failed');
    // Without the shell's server events, the view reads the server again while it starts.
    expect(
      await screen.findByText("The site's dev server did not start", undefined, { timeout: 4000 })
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeVisible();
  });

  it('asks for a site repo when the project has none', async () => {
    await show('no-site');
    expect(await screen.findByRole('heading', { name: 'No site repo yet' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Open in browser' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('opens the compare board with the device verdict', async () => {
    await show('running');
    await screen.findByTitle('The frostveil-site page from the site dev page', undefined, {
      timeout: 4000,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Compare' }));
    expect(screen.getByText('Ship vs run')).toBeVisible();
    expect(await screen.findByRole('complementary', { name: 'Device verdict' })).toBeVisible();
  });
});
