import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openPlans } from '@/test/open-plan';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlansStore } from '@/state/use-plans-store';
import StopEditor from './StopEditor';

beforeEach(() => vi.useFakeTimers({ shouldAdvanceTime: true }));
afterEach(() => {
  vi.useRealTimers();
  clearMocks();
});

const stopOf = (id: string) =>
  usePlansStore.getState().nodes.find(n => n.id === id)?.data.node.stop;

describe('StopEditor', () => {
  it('renames, sets the field of view and moves the selected stop in the main line', async () => {
    await openPlans();
    await usePlanCameraStore.getState().open('frostveil');
    await usePlanCameraStore.getState().refresh('frostveil');
    const [first, second] = usePlanCameraStore.getState().view!.main;
    render(<StopEditor nodeId={second!} />);
    expect(screen.getByRole('heading', { name: 'Stop 2 of 5' })).toBeInTheDocument();
    const name = screen.getByRole('textbox', { name: 'Name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Gate at night{Enter}');
    expect(stopOf(second!)?.name).toBe('Gate at night');
    // An empty name never commits.
    await userEvent.clear(name);
    await userEvent.tab();
    expect(stopOf(second!)?.name).toBe('Gate at night');
    await userEvent.click(screen.getByRole('button', { name: 'Earlier' }));
    const edge = usePlansStore.getState().edges.find(e => e.source === second);
    expect(edge?.target).toBe(first);
  });
});
