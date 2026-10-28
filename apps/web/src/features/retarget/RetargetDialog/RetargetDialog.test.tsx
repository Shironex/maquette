import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import RetargetDialog from './RetargetDialog';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null, busy: false });
});

describe('the retarget dialog', () => {
  it('walks from choosing clips to the review and bakes the version', async () => {
    await openMockLibrary('demo');
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['sword-and-shield-run', 'sword-and-shield-crouch'],
    });
    render(<RetargetDialog />);
    expect(
      screen.getByRole('heading', { name: 'Add library clips to Elsheria' })
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Next: bone map' }));
    await screen.findByRole('button', { name: 'Retarget 2 clips' });
    fireEvent.click(screen.getByRole('button', { name: 'Retarget 2 clips' }));
    await screen.findByRole('heading', { name: 'Review the retarget on Elsheria' });
    expect(screen.getByText(/1 good|0 good/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Bake 1 clip into v3' }));
    await waitFor(() => expect(useRetargetStore.getState().sheet).toBeNull());
  });

  it('asks for a model and clips before the next step', async () => {
    await openMockLibrary('demo');
    await useRetargetStore.getState().open({ project: 'frostveil' });
    render(<RetargetDialog />);
    expect(screen.getByRole('button', { name: 'Next: bone map' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(useRetargetStore.getState().sheet).toBeNull();
  });
});
