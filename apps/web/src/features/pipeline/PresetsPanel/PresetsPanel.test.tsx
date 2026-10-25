import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { PresetsPanel } from '.';

const view = pipelineView('moon-island', '/w', runHistory(0), []);

afterEach(() => usePipelineStore.setState({ presetEdits: {} }));

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

  it('keeps edits that are not applied through new events and a remount, and says so', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    const size = () =>
      (screen.getByRole('combobox', { name: 'Island texture size' }) as HTMLSelectElement).value;
    const { rerender, unmount } = render(<PresetsPanel view={view} onSave={onSave} />);
    const saved = size();
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Island texture size' }),
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
    const select = screen.getByRole('combobox', { name: 'Island codec' }) as HTMLSelectElement;
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
});
