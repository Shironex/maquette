import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PageSource } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the page source', () => {
  it('shows the site dev page and switches to a page concept', async () => {
    await openMockWorkspace();
    await usePageStore.getState().open('moon-island');
    render(<PageSource />);
    expect(screen.getByRole('radio', { name: /Site dev page/ })).toBeChecked();
    await userEvent.click(screen.getByRole('radio', { name: /HTML concept/ }));
    expect(usePageStore.getState().view?.settings.source).toBe('concept');
    expect(await screen.findByText('concepts/home_v2.html')).toBeVisible();
    await userEvent.click(screen.getByRole('radio', { name: /Site dev page/ }));
    expect(usePageStore.getState().view?.settings.source).toBe('site');
  });
});
