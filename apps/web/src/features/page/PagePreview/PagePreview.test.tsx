import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PagePreview } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the page preview', () => {
  it('starts the dev server and shows the page in a small frame', async () => {
    await openMockWorkspace('configured');
    render(<PagePreview />);
    expect(await screen.findByText("Starting the site's dev server")).toBeVisible();
    expect(
      await screen.findByTitle('The frostveil-site page from the site dev page', undefined, {
        timeout: 4000,
      })
    ).toBeInTheDocument();
  });
});
