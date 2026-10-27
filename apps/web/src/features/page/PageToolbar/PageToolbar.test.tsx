import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PageToolbar } from '.';
import { pageTitle } from './PageToolbar.hooks';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

async function open() {
  await openMockWorkspace(
    'configured',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    'running'
  );
  await usePageStore.getState().open('frostveil');
}

describe('pageTitle', () => {
  it('names the page after its path', () => {
    expect(pageTitle('/', false)).toBe('Home page');
    expect(pageTitle('/pricing-plans/', false)).toBe('Pricing plans page');
    expect(pageTitle('/anything', true)).toBe('Page concept');
  });
});

describe('the page toolbar', () => {
  it('sets the device and the overlays', async () => {
    await open();
    render(<PageToolbar />);
    expect(screen.getByRole('heading', { name: 'Home page' })).toBeVisible();
    await userEvent.click(screen.getByText('Phone'));
    expect(usePageStore.getState().device).toBe('phone');
    const stops = screen.getByRole('button', { name: 'Stops' });
    const before = usePageStore.getState().showStops;
    await userEvent.click(stops);
    expect(usePageStore.getState().showStops).toBe(!before);
    await userEvent.click(screen.getByRole('button', { name: 'Poster' }));
    expect(usePageStore.getState().poster).toBe(true);
  });

  it('shows the compare layouts instead of the overlays while comparing', async () => {
    await open();
    act(() => usePageStore.getState().setMode('ship-run'));
    render(<PageToolbar />);
    expect(screen.queryByRole('button', { name: 'Stops' })).toBeNull();
    await userEvent.click(screen.getByText('With poster'));
    expect(usePageStore.getState().mode).toBe('poster');
  });

  it('shows another path on the site', async () => {
    await open();
    render(<PageToolbar />);
    await userEvent.click(screen.getByRole('button', { name: /^Path on the site/ }));
    const field = screen.getByRole('textbox', { name: 'Path on the site' });
    await userEvent.clear(field);
    await userEvent.type(field, '/pricing{Enter}');
    expect(await screen.findByRole('heading', { name: 'Pricing page' })).toBeVisible();
    expect(usePageStore.getState().view?.settings.path).toBe('/pricing');
  });
});
