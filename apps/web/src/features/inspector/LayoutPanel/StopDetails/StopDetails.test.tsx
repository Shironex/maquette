import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import StopDetails from './StopDetails';

beforeEach(async () => {
  await openMockWorkspace();
  await useLayoutStore.getState().load('moon-island');
});
afterEach(() => {
  clearMocks();
  useLayoutStore.setState({
    view: null,
    project: null,
    selection: null,
    preview: null,
    pose: null,
  });
});

function details(index: number) {
  const { draft, view } = useLayoutStore.getState();
  return render(
    <StopDetails
      stop={draft.stops[index]!}
      index={index}
      count={draft.stops.length}
      models={view!.models}
    />
  );
}

describe('the stop details', () => {
  it('names the first stop as the opening view and counts what it sees', () => {
    details(0);
    expect(screen.getByLabelText('Name · the view the page opens on')).toHaveValue('Porch');
    expect(screen.getByRole('button', { name: 'Earlier in the tour' })).toBeDisabled();
    expect(screen.getByRole('switch', { name: /Count what the camera sees/ })).toBeChecked();
    expect(screen.queryByRole('checkbox')).toBeNull();
  });

  it('picks the first view models by hand once counting is off', async () => {
    const { rerender } = details(0);
    await userEvent.click(screen.getByRole('switch', { name: /Count what the camera sees/ }));
    const stop = () => useLayoutStore.getState().draft.stops[0]!;
    expect(stop().models).toEqual(['island', 'shop-house', 'character', 'vending']);
    const { draft, view } = useLayoutStore.getState();
    rerender(<StopDetails stop={draft.stops[0]!} index={0} count={5} models={view!.models} />);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Island' }));
    expect(stop().models).toEqual(['shop-house', 'character', 'vending']);
  });

  it('moves a later stop in the tour, looks through it and deletes it', async () => {
    details(2);
    expect(screen.getByLabelText('Name')).toHaveValue('Shrubs');
    await userEvent.click(screen.getByRole('button', { name: 'Earlier in the tour' }));
    expect(useLayoutStore.getState().draft.stops[1]!.id).toBe('shrubs');
    await userEvent.click(screen.getByRole('button', { name: 'Look through' }));
    expect(useLayoutStore.getState().preview?.id).toBe('shrubs');
    await userEvent.click(screen.getByRole('button', { name: 'Delete Shrubs' }));
    expect(useLayoutStore.getState().draft.stops.map(s => s.id)).not.toContain('shrubs');
  });

  it('cannot take the view until the camera has a pose', () => {
    details(1);
    expect(screen.getByRole('button', { name: 'Set from view' })).toBeDisabled();
  });
});
