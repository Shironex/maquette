import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { SettingsScreen } from '.';

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
});

describe('the settings screen', () => {
  it('says changes save as you go, and Saving soon after an edit', async () => {
    await openMockWorkspace();
    render(<SettingsScreen />);
    const status = screen.getByText('Changes save as you go');
    expect(status).toHaveAttribute('role', 'status');
    const field = screen.getByRole('textbox', { name: 'Scene target' });
    await userEvent.clear(field);
    await userEvent.type(field, '4{Enter}');
    expect(within(status).getByText('Saving soon')).toBeVisible();
  });

  it('moves the nav to the section it jumps to', async () => {
    await openMockWorkspace();
    // jsdom does not scroll.
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    render(<SettingsScreen />);
    const nav = screen.getByRole('navigation', { name: 'Settings sections' });
    await userEvent.click(within(nav).getByRole('button', { name: 'Agents' }));
    expect(scroll).toHaveBeenCalledWith({ block: 'nearest', behavior: 'smooth' });
    expect(within(nav).getByRole('button', { name: 'Agents' })).toHaveAttribute(
      'aria-current',
      'true'
    );
    expect(screen.getByRole('region', { name: 'Agents' })).toHaveClass(
      'shadow-[0_0_0_1px_var(--clay-line)]'
    );
  });
});
