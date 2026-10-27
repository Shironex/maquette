import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useLayoutStore } from '@/state/use-layout-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import LayoutBarActions from './LayoutBarActions';

beforeEach(async () => {
  await openMockWorkspace();
  await useLayoutStore.getState().load('frostveil');
});
afterEach(() => {
  clearMocks();
  useLayoutStore.setState({ view: null, project: null, selection: null });
});

describe('the layout top bar actions', () => {
  it('has nothing to save or revert until the layout changes', () => {
    render(<LayoutBarActions />);
    expect(screen.getByRole('button', { name: 'Save layout' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByRole('button', { name: 'Revert' })).toHaveAttribute('aria-disabled', 'true');
  });

  it('reverts an edit, and saves one', async () => {
    render(<LayoutBarActions />);
    useLayoutStore.getState().setDesk([2, 1]);
    await userEvent.click(await screen.findByRole('button', { name: 'Revert' }));
    expect(useLayoutStore.getState().draft.desk).toBeNull();
    useLayoutStore.getState().setDesk([2, 1]);
    await userEvent.click(await screen.findByRole('button', { name: 'Save layout' }));
    await waitFor(() => expect(useLayoutStore.getState().saved.desk).toEqual([2, 1]));
    expect(screen.getByRole('button', { name: 'Save layout' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});
