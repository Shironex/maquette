import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { CopyButton } from '.';

afterEach(() => useAppStore.setState({ notice: null }));

describe('a copy button', () => {
  it('copies its text and says what was copied', async () => {
    const user = userEvent.setup();
    render(<CopyButton text="guardian-statues-1.glb" what="The file name" />);
    await user.click(screen.getByRole('button', { name: 'Copy The file name' }));
    expect(await navigator.clipboard.readText()).toBe('guardian-statues-1.glb');
    expect(useAppStore.getState().notice?.title).toBe('The file name copied');
  });
});
