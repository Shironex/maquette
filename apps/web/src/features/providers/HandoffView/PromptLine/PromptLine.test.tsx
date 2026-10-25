import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { PromptLine } from '.';

afterEach(() => useAppStore.setState({ notice: null }));

describe('the prompt line', () => {
  it('shows the prompt and copies it in one click', async () => {
    const user = userEvent.setup();
    render(<PromptLine prompt="Pale pink blossoms" />);
    expect(screen.getByText('Pale pink blossoms')).toHaveAttribute('title', 'Pale pink blossoms');
    await user.click(screen.getByRole('button', { name: 'Copy prompt' }));
    expect(await navigator.clipboard.readText()).toBe('Pale pink blossoms');
    expect(useAppStore.getState().notice?.title).toBe('The prompt copied');
  });
});
