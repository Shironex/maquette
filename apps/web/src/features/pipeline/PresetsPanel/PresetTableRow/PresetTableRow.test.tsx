import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { presetRows, type PresetRow } from '@/state/presets';
import { PresetTableRow } from '.';

const island = presetRows(pipelineView('frostveil', '/w', runHistory(0), [])).find(
  r => r.model === 'veil_platform'
)!;

afterEach(cleanup);

function renderRow(row: PresetRow, split = false) {
  const onToggle = vi.fn();
  const onUpdate = vi.fn();
  render(
    <table>
      <tbody>
        <PresetTableRow row={row} split={split} onToggle={onToggle} onUpdate={onUpdate} />
      </tbody>
    </table>
  );
  return { onToggle, onUpdate };
}

describe('a preset row', () => {
  it('opens the split and edits the codec', async () => {
    const { onToggle, onUpdate } = renderRow(island);
    expect(screen.queryByRole('combobox', { name: 'Veil platform data map size' })).toBeNull();
    await userEvent.click(
      screen.getByRole('button', { name: 'Texture sizes per map for Veil platform' })
    );
    expect(onToggle).toHaveBeenCalledWith('veil_platform');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Veil platform codec' }),
      'UASTC'
    );
    expect(onUpdate).toHaveBeenCalledWith('veil_platform', { codec: 'uastc' });
  });

  it('sets the data map size on the open split', async () => {
    const { onUpdate } = renderRow(island, true);
    const data = screen.getByRole('combobox', { name: 'Veil platform data map size' });
    expect(data).toHaveValue('same');
    await userEvent.selectOptions(data, '512 px');
    expect(onUpdate).toHaveBeenCalledWith('veil_platform', { dataTextureSize: 512 });
  });

  it('locks every control of a locked row', () => {
    renderRow({ ...island, lock: 'Kept on medium.' });
    expect(screen.getByRole('combobox', { name: 'Veil platform meshopt level' })).toBeDisabled();
    expect(
      screen.getByRole('button', { name: 'Texture sizes per map for Veil platform' })
    ).toHaveAttribute('aria-disabled', 'true');
  });
});
