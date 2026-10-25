import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useSettingsGuard } from '@/state/settings-guard';
import { InvalidSettingsDialog } from '.';

afterEach(() => useSettingsGuard.setState({ problems: [], leaveTo: null }));

function renderDialog() {
  const discardInvalid = vi.fn();
  useAppStore.setState({ view: 'settings' });
  useSettingsGuard.setState({ leaveTo: 'board' });
  render(
    <InvalidSettingsDialog
      leaveTo="board"
      problems={['Scene target: must be above 0', 'Port: must be a number']}
      discardInvalid={discardInvalid}
    />
  );
  return discardInvalid;
}

describe('the invalid settings question', () => {
  it('lists what is wrong and Fix it stays in Settings', async () => {
    const discardInvalid = renderDialog();
    const dialog = screen.getByRole('dialog', { name: 'Some settings are not valid' });
    expect(dialog).toHaveTextContent('Scene target: must be above 0');
    expect(dialog).toHaveTextContent('Port: must be a number');
    await userEvent.click(screen.getByRole('button', { name: 'Fix it' }));
    expect(useSettingsGuard.getState().leaveTo).toBeNull();
    expect(discardInvalid).not.toHaveBeenCalled();
    expect(useAppStore.getState().view).toBe('settings');
  });

  it('Discard drops the invalid fields and leaves', async () => {
    const discardInvalid = renderDialog();
    await userEvent.click(screen.getByRole('button', { name: 'Discard' }));
    expect(discardInvalid).toHaveBeenCalledTimes(1);
    expect(useAppStore.getState().view).toBe('board');
  });
});
