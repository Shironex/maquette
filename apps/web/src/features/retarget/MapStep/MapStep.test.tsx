import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import MapStep from './MapStep';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null });
});

describe('the bone map step', () => {
  it('shows every humanoid bone and leaves one at rest when picked so', async () => {
    await openMockLibrary('demo');
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['standing-dodge-left'],
      step: 'map',
    });
    render(<MapStep />);
    expect(screen.getByRole('combobox', { name: 'Elsheria bone for LeftArm' })).toBeInTheDocument();
    expect(screen.getByText(/22 matched/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Needs a look' }));
    expect(
      screen.queryByRole('combobox', { name: 'Elsheria bone for LeftArm' })
    ).not.toBeInTheDocument();
    await useRetargetStore.getState().pick('Neck', null);
    await waitFor(() => expect(screen.getByText('unmapped, stays at rest')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('checkbox', { name: /Save this map for Elsheria/ }));
    expect(useRetargetStore.getState().sheet!.saveMap).toBe(false);
  });
});
