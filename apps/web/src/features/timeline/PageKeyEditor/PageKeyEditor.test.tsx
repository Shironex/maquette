import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { act } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { useTimelineStore } from '@/state/use-timeline-store';
import { clearMocks, openTimeline } from '@/test/open-timeline';
import { PageKeyEditor } from '.';

afterEach(clearMocks);

describe('the page key editor', () => {
  it('changes the look and removes the track', async () => {
    await openTimeline('looks');
    act(() =>
      useTimelineStore.getState().select({ kind: 'page', lane: 'style', stop: 'keystone' })
    );
    render(<PageKeyEditor />);
    expect(screen.getByRole('heading', { name: 'Render style at Keystone' })).toBeVisible();
    expect(screen.getByText('A filter on the 3D layer')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Remove the track' }));
    const s = useTimelineStore.getState();
    expect(s.draft.timeline.style).toBeNull();
    expect(s.selection).toBeNull();
  });

  it('says what the hero text fades, without a look', async () => {
    await openTimeline('looks');
    act(() => useTimelineStore.getState().select({ kind: 'page', lane: 'text', stop: 'arrival' }));
    render(<PageKeyEditor />);
    expect(screen.getByText('Copy marked data-maquette-text')).toBeVisible();
    expect(screen.queryByLabelText('Look')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Remove this key' }));
    const keys = useTimelineStore.getState().draft.timeline.text?.keys;
    expect(keys).toEqual([{ stop: 'veil-gate', amount: 0 }]);
  });
});
