import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelineView } from '.';

const INITIAL = usePageStore.getState();

afterEach(() => {
  clearMocks();
  usePageStore.setState(INITIAL, true);
});

describe('the timeline view', () => {
  it('lays out the camera, the cabinet lanes and the paths still to come', async () => {
    await openTimeline();
    render(<TimelineView preview={<p>The page</p>} />);
    expect(await screen.findByRole('heading', { name: 'Scroll timeline' })).toBeVisible();
    expect(screen.getByText('The page')).toBeVisible();
    expect(screen.getByRole('button', { name: /^Camera stop 2, Sakura, at 25%$/ })).toBeVisible();
    expect(screen.getByRole('group', { name: 'Switch cabinet states' })).toBeVisible();
    expect(screen.getByRole('group', { name: 'Switch cabinet explode' })).toBeVisible();
    expect(screen.getByText('Paths come in the next view')).toBeVisible();
    expect(screen.getByText('Saved to layout.json as you go')).toBeVisible();
  });

  it('opens the Page view from the preview', async () => {
    await openTimeline();
    render(<TimelineView preview={null} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Open on the page' }));
    expect(useAppStore.getState().view).toBe('page');
  });

  it('says what to do when the project has no camera stops', async () => {
    await openTimeline('empty');
    render(<TimelineView preview={null} />);
    expect(await screen.findByRole('heading', { name: /has no camera stops yet$/ })).toBeVisible();
    expect(screen.queryByRole('region', { name: 'Scroll timeline' })).toBeNull();
  });
});
