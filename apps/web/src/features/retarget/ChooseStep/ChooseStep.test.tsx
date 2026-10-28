import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useRetargetStore } from '@/state/use-retarget-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import ChooseStep from './ChooseStep';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null });
});

describe('choosing clips', () => {
  it('filters by pack, chooses clips and totals what they add', async () => {
    await openMockLibrary('demo');
    await useRetargetStore.getState().open({ project: 'frostveil', asset: 'elsheria' });
    render(<ChooseStep />);
    await screen.findByRole('checkbox', { name: 'Standing Dodge Left' });
    const packs = screen.getByRole('group', { name: 'Packs' });
    fireEvent.click(within(packs).getByRole('button', { name: /Pro Magic Pack/ }));
    expect(screen.queryByRole('checkbox', { name: 'Standing Dodge Left' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Magic Idle' }));
    await waitFor(() => expect(useRetargetStore.getState().sheet!.chosen).toEqual(['magic-idle']));
    expect(screen.getByText('Chosen · 1')).toBeInTheDocument();
    expect(screen.getByText('90 frames')).toBeInTheDocument();
  });
});
