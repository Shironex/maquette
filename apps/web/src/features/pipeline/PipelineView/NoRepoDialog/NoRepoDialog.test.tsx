import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { NoRepoDialog } from '.';

describe('NoRepoDialog', () => {
  it('names what the run will not do and offers both ways on', async () => {
    const onRun = vi.fn();
    const onSettings = vi.fn();
    render(<NoRepoDialog repo="site" onRun={onRun} onSettings={onSettings} onDismiss={vi.fn()} />);
    expect(
      screen.getByText(/no site repo, so the run builds everything and ships nothing/)
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Run, ship nothing' }));
    expect(onRun).toHaveBeenCalledOnce();
    await userEvent.click(screen.getByRole('button', { name: 'Open Settings' }));
    expect(onSettings).toHaveBeenCalledOnce();
  });
});
