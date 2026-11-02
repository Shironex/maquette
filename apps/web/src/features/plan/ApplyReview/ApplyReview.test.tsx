import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type CameraReview } from '@/ipc/client';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlansStore } from '@/state/use-plans-store';
import ApplyReview from './ApplyReview';

const line = (kind: 'add' | 'remove' | 'same', id: string, note: string | null) => ({
  kind,
  text: `{ "id": "${id}" }`,
  note,
});

/** Moon moves from 4 to 2, a lantern comes, the browser goes. */
const review: CameraReview = {
  diff: {
    stops: [
      {
        id: 'porch',
        name: 'Porch',
        node: 'n1',
        from: 1,
        to: 1,
        added: false,
        removed: false,
        moved: false,
        fields: [],
      },
      {
        id: 'moon',
        name: 'Moon',
        node: 'n4',
        from: 4,
        to: 2,
        added: false,
        removed: false,
        moved: true,
        fields: [{ field: 'fov', before: '35', after: '28' }],
      },
      {
        id: 'lantern',
        name: 'Lantern',
        node: 'n9',
        from: null,
        to: 3,
        added: true,
        removed: false,
        moved: false,
        fields: [],
      },
      {
        id: 'browser',
        name: 'Browser',
        node: null,
        from: 3,
        to: null,
        added: false,
        removed: true,
        moved: false,
        fields: [],
      },
    ],
    lines: [
      line('same', 'porch', null),
      line('add', 'moon', 'moved from 4'),
      line('add', 'lantern', 'new'),
      line('remove', 'browser', 'removed'),
      line('remove', 'moon', 'moves to 2'),
    ],
    added: 1,
    removed: 1,
    moved: 1,
    changed: 1,
  },
  chain: ['n1', 'n4', 'n9'],
  dropped: 2,
  stale: false,
  problems: [],
  layoutKey: 'lk',
  planKey: 'pk',
};

afterEach(() => {
  vi.restoreAllMocks();
  usePlanCameraStore.getState().clear();
});

const show = (error: string | null = null) => {
  usePlansStore.setState({ project: 'moon-island' });
  usePlanCameraStore.setState({
    project: 'moon-island',
    apply: { review, error, applying: false },
  });
  render(<ApplyReview />);
};

describe('ApplyReview', () => {
  it('shows every real change, the moved stop leaving and coming in at its new place', () => {
    show();
    const dialog = screen.getByRole('dialog', { name: 'Apply to layout.json' });
    expect(dialog).toHaveTextContent('1 stop added · 1 moved · 1 removed · 1 changed');
    const order = within(screen.getByRole('region', { name: 'Order this Apply writes' }));
    expect(order.getByText('Moon').closest('tr')).toHaveTextContent('moves from 4; fov 35 to 28');
    expect(order.getByText('Lantern').closest('tr')).toHaveTextContent('new stop');
    expect(order.getByText('Browser').closest('tr')).toHaveTextContent('removed');
    const changes = screen.getByRole('region', { name: 'Changes to layout.json' });
    expect(changes).toHaveTextContent('moved from 4');
    expect(changes).toHaveTextContent('moves to 2');
    expect(changes).toHaveTextContent('2 triggers, timeline or path keys wait for a removed stop');
  });

  it('applies with the keys it was shown with', async () => {
    vi.spyOn(usePlansStore.getState(), 'save').mockResolvedValue(true);
    const apply = vi.spyOn(commands, 'planCameraApply').mockRejectedValue({
      code: 'invalid',
      message: 'layout.json changed since this review was shown, so nothing was applied.',
    });
    show();
    await userEvent.click(screen.getByRole('button', { name: 'Apply' }));
    expect(apply).toHaveBeenCalledWith('moon-island', 'lk', 'pk');
    // A stale review is refused and says to review again; Apply waits for that.
    expect(await screen.findByText('Nothing was applied')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Review again' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Apply' })).toHaveAttribute('aria-disabled', 'true');
  });
});
