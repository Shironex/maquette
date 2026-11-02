import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ReactFlowProvider } from '@xyflow/react';
import { afterEach, beforeAll, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePlanLinksStore } from '@/state/use-plan-links-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, fakeFlowLayout, openPlans } from '@/test/open-plan';
import { PlanCanvas } from '../../PlanCanvas';

beforeAll(fakeFlowLayout);
afterEach(() => {
  usePlanLinksStore.getState().clear();
  clearMocks();
});

/** Boss: Frost Warden links a direction and five assets, each in another state. */
const renderBoss = async () => {
  await openPlans();
  await usePlansStore.getState().openCanvas('boss-frost-warden');
  render(
    <ReactFlowProvider>
      <PlanCanvas />
    </ReactFlowProvider>
  );
};

const node = (label: string) => screen.getByRole('group', { name: label });

describe('LinkedBody', () => {
  it('shows each linked asset live, with its state and next step', async () => {
    await renderBoss();
    const elsheria = await screen.findByRole('group', { name: 'Asset: Elsheria' });
    expect(within(elsheria).getByText(/Model ready, v\d/)).toBeInTheDocument();
    expect(within(elsheria).getByRole('button', { name: 'Open in Inspector' })).toBeVisible();

    const gate = node('Asset: Veil gate');
    expect(within(gate).getByText('No model yet')).toBeInTheDocument();
    expect(within(gate).getByRole('button', { name: 'Open Lab' })).toBeVisible();

    const obelisk = node('Asset: Rune obelisk');
    expect(within(obelisk).getByText('Model to check')).toBeInTheDocument();
    expect(within(obelisk).getByText('1 model in the Inbox')).toBeInTheDocument();

    const arch = node('Asset: Observatory arch');
    expect(within(arch).getByText('File missing')).toBeInTheDocument();
  });

  it('shows a removed asset as not found, not as an error', async () => {
    await renderBoss();
    const gone = await screen.findByRole('group', { name: 'Asset: Asset not found' });
    expect(within(gone).getByText('Not on the Board')).toBeInTheDocument();
    expect(within(gone).getByText('frost-warden')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('shows a Lab direction with its picks and opens the Lab on it', async () => {
    await renderBoss();
    const lake = await screen.findByRole('group', { name: 'Lab direction: Frozen lake arena' });
    expect(within(lake).getByText('Picked')).toBeInTheDocument();
    await userEvent.click(within(lake).getByRole('button', { name: 'Open in Lab' }));
    expect(useAppStore.getState().view).toBe('lab');
  });

  it('opens the Inbox for a model to check', async () => {
    await renderBoss();
    const obelisk = await screen.findByRole('group', { name: 'Asset: Rune obelisk' });
    await userEvent.click(within(obelisk).getByRole('button', { name: 'Open Inbox' }));
    expect(useAppStore.getState().view).toBe('inbox');
  });
});
