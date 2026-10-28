import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAnimationsStore } from '@/state/use-animations-store';
import { AnimationDialogs } from '.';

afterEach(() => useAnimationsStore.setState({ importing: null, saving: null }));

describe('the animation dialogs', () => {
  it('shows nothing until a sheet opens, then that sheet', () => {
    const { rerender } = render(<AnimationDialogs />);
    expect(screen.queryByRole('dialog')).toBeNull();
    useAnimationsStore.getState().openImport();
    rerender(<AnimationDialogs />);
    expect(screen.getByRole('dialog', { name: 'Import clips' })).toBeVisible();
  });
});
