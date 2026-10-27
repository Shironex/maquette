import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type InspectModel } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import MapCodecRows from './MapCodecRows';

let elsheria: InspectModel;
let gate: InspectModel;

beforeEach(async () => {
  await openMockWorkspace();
  const view = await commands.inspectProject('frostveil');
  elsheria = view.models.find(m => m.model === 'elsheria')!;
  gate = view.models.find(m => m.model === 'veil_gate')!;
});
afterEach(() => clearMocks());

describe('the per map codec rows', () => {
  it("shows each group's codec and what its maps weigh as built", () => {
    render(<MapCodecRows model={gate} onCodec={vi.fn()} />);
    expect(screen.getByText('1 map, 190 KB ETC1S')).toBeInTheDocument();
    expect(screen.getByText('1 map, 23 KB ETC1S')).toBeInTheDocument();
    expect(screen.getByText('1 map, 39 KB ETC1S')).toBeInTheDocument();
  });

  it("keeps a locked row's normal maps on its codec, and says which maps a model lacks", () => {
    render(<MapCodecRows model={elsheria} onCodec={vi.fn()} />);
    // Elsheria's row is locked, so her normal maps keep the row's ETC1S.
    const normal = screen.getByRole('group', { name: 'Normal map codec' });
    expect(within(normal).getByRole('button', { name: 'ETC1S' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(screen.getByText('1 map, 225 KB ETC1S')).toBeInTheDocument();
    expect(screen.getAllByText('none in this model')).toHaveLength(2);
  });

  it('marks a group whose built maps a run would change', () => {
    // The gate's row is not locked, so its normal maps go to UASTC on the next run.
    expect(gate.preset.codecs.normal).toBe('uastc');
    render(<MapCodecRows model={gate} onCodec={vi.fn()} />);
    expect(screen.getByText('1 map, 23 KB ETC1S')).toHaveAttribute(
      'title',
      'Built as ETC1S. Compress encodes them as UASTC on the next run.'
    );
    expect(screen.getByText('1 map, 190 KB ETC1S')).toHaveAttribute(
      'title',
      'Built as ETC1S, as chosen.'
    );
  });

  it('saves a group codec, and does nothing while the row is locked', async () => {
    const onCodec = vi.fn();
    const { unmount } = render(<MapCodecRows model={gate} onCodec={onCodec} />);
    const normal = screen.getByRole('group', { name: 'Normal map codec' });
    await userEvent.click(within(normal).getByRole('button', { name: 'ETC1S' }));
    expect(onCodec).toHaveBeenCalledWith('normal', 'etc1s');
    unmount();

    render(<MapCodecRows model={elsheria} onCodec={null} />);
    expect(screen.getByRole('group', { name: 'Color map codec' })).toHaveClass(
      'pointer-events-none'
    );
  });
});
