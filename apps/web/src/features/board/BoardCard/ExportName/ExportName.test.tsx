import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { ExportName } from '.';

afterEach(() => useAppStore.setState({ notice: null }));

describe('the export name on a card', () => {
  it('shows the name and copies it from its button', async () => {
    const user = userEvent.setup();
    render(<ExportName name="maq_iron-street-lamp_v1_tripo" />);
    expect(screen.getByText('maq_iron-street-lamp_v1_tripo')).toHaveClass('truncate');
    await user.click(screen.getByRole('button', { name: 'Copy export name' }));
    expect(await navigator.clipboard.readText()).toBe('maq_iron-street-lamp_v1_tripo');
    expect(useAppStore.getState().notice?.title).toBe('Export name copied');
  });
});
