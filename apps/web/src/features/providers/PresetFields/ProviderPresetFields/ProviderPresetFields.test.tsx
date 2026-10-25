import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ProviderPresetFields } from '.';

afterEach(() => clearMocks());

describe('the saved preset fields', () => {
  it('switch to the Meshy preset and save its change into the config', async () => {
    await openMockWorkspace();
    const config = defaultConfig();
    const onChange = vi.fn();
    render(<ProviderPresetFields config={config} onChange={onChange} />);
    await userEvent.click(
      within(screen.getByRole('group', { name: 'Service' })).getByRole('button', { name: 'Meshy' })
    );
    await userEvent.click(screen.getByRole('switch', { name: 'Multi-view' }));
    const meshy = config.providers.meshy;
    expect(onChange).toHaveBeenCalledWith({
      ...config,
      providers: {
        ...config.providers,
        meshy: { ...meshy, prop: { ...meshy.prop, multiView: !meshy.prop.multiView } },
      },
    });
  });

  it('show the error of the preset in view', async () => {
    await openMockWorkspace();
    render(
      <ProviderPresetFields
        config={defaultConfig()}
        onChange={() => {}}
        errors={{ 'tripo-prop': 'Face limit out of range' }}
      />
    );
    expect(screen.getByText('Face limit out of range')).toBeInTheDocument();
  });
});
