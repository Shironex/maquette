import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type InspectModel, type InspectView } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import ModelPanel, { type ModelPanelProps } from './ModelPanel';

let view: InspectView;
let elsheria: InspectModel;

beforeEach(async () => {
  await openMockWorkspace();
  view = await commands.inspectProject('frostveil');
  elsheria = view.models.find(m => m.model === 'elsheria')!;
});
afterEach(() => clearMocks());

function panel(over: Partial<ModelPanelProps> = {}) {
  const props: ModelPanelProps = {
    view,
    model: elsheria,
    thumb: null,
    onPickBudget: vi.fn(),
    onPreset: vi.fn(),
    onExport: vi.fn(),
    exporting: false,
    ...over,
  };
  render(<ModelPanel {...props} />);
  return props;
}

describe('the model panel', () => {
  it('checks a game model against its folder budget, with no compression or scene total', () => {
    panel({ game: { triangles: 1_000 } });
    const tris = screen.getByText('Triangles').nextElementSibling!;
    expect(tris).toHaveAttribute('title', "Over its folder's 1,000 triangle budget");
    expect(screen.queryByRole('heading', { name: 'Compression' })).toBeNull();
    expect(screen.queryByText('Scene total')).toBeNull();
    expect(screen.queryByText('Model budgets')).toBeNull();
    expect(screen.getByText(/There is no scene total/)).toBeInTheDocument();
  });

  it('asks for a model when none is picked', () => {
    panel({ model: null });
    expect(
      screen.getByText(/Click a model in the view, or a budget row below/)
    ).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Compression' })).toBeNull();
  });

  it("shows the model's numbers, textures and every model's budget", () => {
    panel();
    expect(screen.getByRole('heading', { name: 'Compression' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Textures' })).toBeInTheDocument();
    expect(screen.getAllByRole('meter')).toHaveLength(view.models.length);
    expect(screen.getByRole('button', { name: /^Elsheria/ })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });

  it('saves a new texture codec as the model preset', async () => {
    const free = view.models.find(m => !m.preset.lock && !m.error)!;
    const props = panel({ model: free });
    const other = free.preset.codec === 'etc1s' ? 'UASTC' : 'KTX2 ETC1S';
    const codec = screen.getByRole('group', { name: 'Texture codec' });
    await userEvent.click(within(codec).getByRole('button', { name: other }));
    expect(props.onPreset).toHaveBeenCalledWith(
      expect.objectContaining({
        model: free.model,
        codec: other === 'UASTC' ? 'uastc' : 'etc1s',
        lock: null,
      })
    );
  });

  it('saves a map group codec into the row, keeping the others', async () => {
    const gate = view.models.find(m => m.model === 'veil_gate')!;
    const props = panel({ model: gate });
    const normal = screen.getByRole('group', { name: 'Normal map codec' });
    await userEvent.click(within(normal).getByRole('button', { name: 'ETC1S' }));
    expect(props.onPreset).toHaveBeenCalledWith(
      expect.objectContaining({
        model: 'veil_gate',
        codec: gate.preset.codec,
        mapCodecs: { color: null, normal: 'etc1s', other: null },
      })
    );
  });

  it('shows the material tweaks for the model', () => {
    panel();
    expect(screen.getByRole('heading', { name: 'Material' })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Applies to' })).toBeInTheDocument();
  });

  it('picks a budget row and exports the model', async () => {
    const props = panel();
    await userEvent.click(screen.getByRole('button', { name: /^Veil gate/ }));
    expect(props.onPickBudget).toHaveBeenCalledWith('veil_gate');
    await userEvent.click(screen.getByRole('button', { name: 'Export GLB' }));
    expect(props.onExport).toHaveBeenCalledWith(elsheria);
  });
});
