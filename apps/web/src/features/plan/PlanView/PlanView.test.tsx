import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { usePlansStore } from '@/state/use-plans-store';
import { clearMocks, fakeFlowLayout, openPlans } from '@/test/open-plan';
import PlanView from './PlanView';

beforeAll(fakeFlowLayout);
afterEach(clearMocks);

describe('PlanView', () => {
  it('lists the canvases, opens one and goes back', async () => {
    await openPlans();
    render(<PlanView />);
    expect(await screen.findByRole('region', { name: 'All canvases' })).toBeInTheDocument();
    expect(screen.getByText('4 canvases')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Combat ideas' }));
    expect(await screen.findByRole('main', { name: 'Combat ideas canvas' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Plan, back to all canvases' }));
    expect(await screen.findByRole('region', { name: 'All canvases' })).toBeInTheDocument();
  });

  it('says when the canvases cannot be read and reads them again', async () => {
    await openPlans();
    const list = vi
      .spyOn(commands, 'planList')
      .mockRejectedValueOnce({ code: 'io', message: 'The plans folder is locked' });
    usePlansStore.setState({ list: null });
    render(<PlanView />);
    expect(await screen.findByText('The plans folder is locked')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('region', { name: 'All canvases' })).toBeInTheDocument();
    expect(list).toHaveBeenCalledTimes(2);
  });

  it('says when the folder cannot be watched and reads the canvas again on focus', async () => {
    await openPlans();
    vi.spyOn(commands, 'planWatch').mockRejectedValue({ code: 'io', message: 'Too many watches' });
    render(<PlanView />);
    await userEvent.click(await screen.findByRole('button', { name: 'Combat ideas' }));
    await waitFor(() => expect(useAppStore.getState().notice?.detail).toMatch(/Too many watches/));
    const reload = vi.spyOn(usePlansStore.getState(), 'reload');
    usePlansStore.setState({ reload });
    act(() => void window.dispatchEvent(new Event('focus')));
    expect(reload).toHaveBeenCalled();
  });
});
