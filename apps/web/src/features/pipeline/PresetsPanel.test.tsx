import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { PresetsPanel } from './PresetsPanel';

const view = pipelineView('moon-island', '/w', runHistory(0), []);

describe('the compression presets', () => {
  it('keeps one size per model until the split is opened, then saves both', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<PresetsPanel view={view} onSave={onSave} />);
    expect(screen.queryByRole('combobox', { name: 'Island data map size' })).toBeNull();
    const before = screen.getByText(/B$/, { selector: '.font-mono' }).textContent;

    await userEvent.click(screen.getByRole('button', { name: 'Texture sizes per map for Island' }));
    const data = screen.getByRole('combobox', { name: 'Island data map size' });
    await userEvent.selectOptions(data, '512 px');
    expect(screen.getByText(/B$/, { selector: '.font-mono' }).textContent).not.toBe(before);

    await userEvent.click(screen.getByRole('button', { name: 'Apply presets' }));
    const saved = onSave.mock.calls[0]![0] as { model: string; dataTextureSize: number | null }[];
    expect(saved.find(p => p.model === 'island')!.dataTextureSize).toBe(512);
    expect(saved.find(p => p.model === 'sakura')!.dataTextureSize).toBeNull();
  });

  it('opens the extra line for a row that already splits its sizes', () => {
    const split = {
      ...view,
      def: {
        ...view.def,
        presets: view.def.presets.map(p =>
          p.model === 'sakura' ? { ...p, dataTextureSize: 512 } : p
        ),
      },
    };
    render(<PresetsPanel view={split} onSave={vi.fn()} />);
    const data = screen.getByRole('combobox', { name: 'Sakura data map size' });
    expect(within(data.parentElement!).getByText(/512/)).toBeInTheDocument();
  });
});
