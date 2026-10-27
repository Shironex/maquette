import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { SyncHelp } from '.';

describe('the sync help', () => {
  it('shows and copies the recommended .stignore', async () => {
    const user = userEvent.setup();
    render(<SyncHelp cache="cache" />);
    await user.click(screen.getByText('Sync with another computer'));
    expect(screen.getByText(/Use Maquette on one computer at a time/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Copy .stignore' }));
    expect(await navigator.clipboard.readText()).toContain('/cache\n');
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();
  });
});
