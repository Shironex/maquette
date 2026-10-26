import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { TimelineEmpty } from '.';

afterEach(clearMocks);

describe('the timeline without camera stops', () => {
  it('points at the Inspector, where the stops are made', async () => {
    await openTimeline('empty');
    render(<TimelineEmpty />);
    expect(screen.getByRole('heading', { name: /has no camera stops yet$/ })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Open the Inspector' }));
    expect(useAppStore.getState().view).toBe('inspector');
  });
});
