import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { ReactFlowProvider } from '@xyflow/react';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { usePlanExtrasStore } from '@/state/use-plan-extras-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, fakeFlowLayout, openPlans } from '@/test/open-plan';
import PlanCanvas from '../../PlanCanvas/PlanCanvas';

beforeAll(fakeFlowLayout);
afterEach(clearMocks);

const canvas = async (id: string) => {
  const backend = await openPlans('systems');
  await usePlansStore.getState().openCanvas(id);
  render(
    <ReactFlowProvider>
      <PlanCanvas />
    </ReactFlowProvider>
  );
  await screen.findByRole('main', { name: `${usePlansStore.getState().open?.plan.name} canvas` });
  return backend;
};

describe('NodeExtras', () => {
  it('flags system cards that overlap, by the other card and the word', async () => {
    await canvas('combat-systems');
    const dash = screen.getByRole('group', { name: /^Note: System: Dash/ });
    expect(within(dash).getByText('Same key as System: Frost veil (shift)')).toBeInTheDocument();
    const strike = screen.getByRole('group', { name: /^Note: System: Strike/ });
    expect(within(strike).queryByText(/^Same /)).not.toBeInTheDocument();
  });

  it('marks a node an agent asked to remove', async () => {
    const backend = await canvas('combat-ideas');
    await backend.mock('mock_plan_propose', {
      project: 'frostveil',
      id: 'combat-ideas',
      nodes: ['n-06'],
      reason: 'Answered on the Frost veil card.',
    });
    await usePlanExtrasStore.getState().load('frostveil', 'combat-ideas');
    expect(await screen.findByText('asked to remove')).toBeInTheDocument();
  });

  it('starts a canvas from the selected node and opens it', async () => {
    await canvas('combat-ideas');
    // Selected as a click would (jsdom has no pointer for React Flow's drag).
    act(() =>
      usePlansStore.setState(s => ({
        nodes: s.nodes.map(n => ({ ...n, selected: n.id === 'n-01' })),
      }))
    );
    // A click, not a press: inside the flow a press starts React Flow's pan, which jsdom lacks.
    fireEvent.click(await screen.findByRole('button', { name: 'Start a canvas from this' }));
    await screen.findByRole('heading', { name: 'Frost veil' });
    const open = usePlansStore.getState().open;
    expect(open?.id).toBe('frost-veil');
    expect(open?.plan.nodes).toHaveLength(2);
    expect(usePlansStore.getState().list?.some(p => p.id === 'frost-veil')).toBe(true);
  });
});
