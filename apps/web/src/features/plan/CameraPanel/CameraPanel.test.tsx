import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openPlans } from '@/test/open-plan';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlansStore } from '@/state/use-plans-store';
import CameraPanel from './CameraPanel';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

async function openCamera(bare = false) {
  const store = await openPlans();
  if (bare) await store.mock('mock_layout_bare', { project: 'frostveil' });
  await usePlanCameraStore.getState().open('frostveil');
  render(<CameraPanel hidden={false} onClose={() => undefined} />);
  await waitFor(() => expect(usePlanCameraStore.getState().review).not.toBeNull());
  return store;
}

describe('CameraPanel', () => {
  it('lists the order Apply would write and keeps Apply off while nothing differs', async () => {
    await openCamera();
    const order = within(screen.getByRole('region', { name: 'Order after Apply' }));
    expect(order.getAllByRole('listitem')).toHaveLength(5);
    expect(order.getByText('Arrival')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Apply to layout.json' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('adds a stop at the end, and Apply opens the review', async () => {
    await openCamera();
    await userEvent.click(screen.getByRole('button', { name: 'Add stop' }));
    expect(usePlansStore.getState().nodes.filter(n => n.data.node.stop)).toHaveLength(6);
    await usePlansStore.getState().save();
    await usePlanCameraStore.getState().refresh('frostveil');
    await userEvent.click(await screen.findByRole('button', { name: 'Apply to layout.json' }));
    const dialog = await screen.findByRole('dialog', { name: 'Apply to layout.json' });
    expect(dialog).toHaveTextContent('1 stop added');
  });

  it('says when layout.json changed underneath and offers compare, rebase and start over', async () => {
    const store = await openCamera();
    await store.mock('mock_layout_outside', { project: 'frostveil' });
    await waitFor(() => expect(usePlanCameraStore.getState().view?.stale).toBe(true));
    expect(
      await screen.findByText('layout.json changed since this plan was based on it')
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Compare' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Rebase' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start over' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Apply to layout.json' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('says there are no stops yet, and that a new stop has no render until the pipeline makes one', async () => {
    await openCamera(true);
    const order = screen.getByRole('region', { name: 'Order after Apply' });
    expect(order).toHaveTextContent('No stops yet. Add stop puts the first one in the tour');
    await userEvent.click(screen.getByRole('button', { name: 'Add stop' }));
    await usePlansStore.getState().save();
    await usePlanCameraStore.getState().refresh('frostveil');
    expect(await within(order).findByText(/One stop has no render yet/)).toHaveTextContent(
      'Capture posters'
    );
  });
});
