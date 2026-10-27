import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { PresetsPanel } from '.';

const view = pipelineView('frostveil', '/w', runHistory(0), []);

afterEach(() => usePipelineStore.setState({ presetEdits: {} }));

describe('the compression presets', () => {
  it('keeps one size per model until the split is opened, then saves both', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<PresetsPanel view={view} onSave={onSave} />);
    expect(screen.queryByRole('combobox', { name: 'Veil platform data map size' })).toBeNull();
    const transfer = () => screen.getByText('Est. transfer after Compress').nextElementSibling;
    const before = transfer()!.textContent;

    await userEvent.click(
      screen.getByRole('button', { name: 'Texture sizes per map for Veil platform' })
    );
    const data = screen.getByRole('combobox', { name: 'Veil platform data map size' });
    await userEvent.selectOptions(data, '512 px');
    expect(transfer()!.textContent).not.toBe(before);

    await userEvent.click(screen.getByRole('button', { name: 'Apply presets' }));
    const saved = onSave.mock.calls[0]![0] as { model: string; dataTextureSize: number | null }[];
    expect(saved.find(p => p.model === 'veil_platform')!.dataTextureSize).toBe(512);
    expect(saved.find(p => p.model === 'veil_gate')!.dataTextureSize).toBeNull();
  });

  it('opens the extra line for a row that already splits its sizes', () => {
    const split = {
      ...view,
      def: {
        ...view.def,
        presets: view.def.presets.map(p =>
          p.model === 'veil_gate' ? { ...p, dataTextureSize: 512 } : p
        ),
      },
    };
    render(<PresetsPanel view={split} onSave={vi.fn()} />);
    const data = screen.getByRole('combobox', { name: 'Veil gate data map size' });
    expect(within(data.parentElement!).getByText(/512/)).toBeInTheDocument();
  });

  it('keeps edits that are not applied through new events and a remount, and says so', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    const size = () =>
      (screen.getByRole('combobox', { name: 'Veil platform texture size' }) as HTMLSelectElement)
        .value;
    const { rerender, unmount } = render(<PresetsPanel view={view} onSave={onSave} />);
    const saved = size();
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Veil platform texture size' }),
      saved === '512' ? '1024' : '512'
    );
    const edited = size();
    expect(screen.getByRole('status')).toHaveTextContent('Not applied yet');

    // A log line from a run hands the panel a new view with the same presets.
    rerender(<PresetsPanel view={{ ...view, log: [...view.log] }} onSave={onSave} />);
    expect(size()).toBe(edited);
    // Board and back.
    unmount();
    render(<PresetsPanel view={view} onSave={onSave} />);
    expect(size()).toBe(edited);
    expect(screen.getByRole('status')).toHaveTextContent('Not applied yet');

    await userEvent.click(screen.getByRole('button', { name: 'Apply presets' }));
    expect(onSave).toHaveBeenCalledOnce();
    expect(usePipelineStore.getState().presetEdits).toEqual({});
  });

  it('drops the hint when an edit goes back to the saved value', async () => {
    render(<PresetsPanel view={view} onSave={vi.fn()} />);
    const select = screen.getByRole('combobox', {
      name: 'Veil platform codec',
    }) as HTMLSelectElement;
    const saved = select.value;
    await userEvent.selectOptions(select, saved === 'etc1s' ? 'UASTC' : 'ETC1S');
    expect(screen.getByRole('status')).toHaveTextContent('Not applied yet');
    await userEvent.selectOptions(select, saved === 'etc1s' ? 'ETC1S' : 'UASTC');
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.getByRole('button', { name: 'Apply presets' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('shows what each model shipped at, sorts by size and links to the size report', async () => {
    const onSizeReport = vi.fn();
    render(<PresetsPanel view={view} onSave={vi.fn()} onSizeReport={onSizeReport} />);
    expect(screen.getByText('4 models · 14.3 MB → 3.8 MB')).toBeInTheDocument();
    const names = () =>
      screen
        .getAllByRole('button', { name: /^Texture sizes per map for / })
        .map(b => b.getAttribute('aria-label')!.replace('Texture sizes per map for ', ''));
    expect(names()[0]).toBe('Character');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Sort the models by' }),
      'By raw size'
    );
    expect(names().slice(0, 2)).toEqual(['Character', 'Island']);
    expect(names().at(-1)).toBe('Sakura');
    const island = screen.getByRole('button', { name: 'Texture sizes per map for Island' });
    expect(within(island.closest('tr')!).getByText('1.1 MB')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Size report' }));
    expect(onSizeReport).toHaveBeenCalledOnce();
  });
});
