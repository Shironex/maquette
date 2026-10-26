import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { PathKeyEditor } from '.';

afterEach(() => {
  clearMocks();
  useInspectorStore.setState({ tab: 'compare' });
});

const select = (stop: string) =>
  useTimelineStore.getState().select({ kind: 'path', path: 'lantern-cable', stop });

const track = () => useTimelineStore.getState().draft.timeline.paths['lantern-cable'];

describe('the energize key editor', () => {
  it('sets how much of the path is lit at the stop and the easing', async () => {
    await openTimeline();
    select('canopy');
    render(<PathKeyEditor />);
    expect(
      screen.getByRole('heading', { name: 'Lantern cable · energize at Canopy' })
    ).toBeVisible();
    const amount = screen.getByLabelText('Lit at Canopy');
    await userEvent.clear(amount);
    await userEvent.type(amount, '60{Enter}');
    expect(track()?.keys).toContainEqual({ stop: 'canopy', amount: 0.6 });
    await userEvent.selectOptions(screen.getByLabelText('Easing'), 'ease-in-out');
    expect(track()?.easing).toBe('ease-in-out');
  });

  it('says a stop is not keyed yet, opens the path and removes the track', async () => {
    await openTimeline();
    select('porch');
    render(<PathKeyEditor />);
    expect(
      screen.getByText('Not keyed at Porch yet. Set how much is lit to key it.')
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Edit the path' }));
    expect(useInspectorStore.getState().tab).toBe('paths');
    expect(useAppStore.getState().view).toBe('inspector');
    await userEvent.click(screen.getByRole('button', { name: 'Remove the track' }));
    expect(track()).toBeUndefined();
  });
});
