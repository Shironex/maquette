import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type InspectModel } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import MapCodecRows from './MapCodecRows';

let character: InspectModel;

beforeEach(async () => {
  await openMockWorkspace();
  const view = await commands.inspectProject('moon-island');
  character = view.models.find(m => m.model === 'character')!;
});
afterEach(() => clearMocks());

describe('the per map codec rows', () => {
  it("shows each group's codec and what its maps weigh as built", () => {
    render(<MapCodecRows model={character} onCodec={vi.fn()} />);
    // The character's row is locked, so its normal maps keep the row's ETC1S.
    const normal = screen.getByRole('group', { name: 'Normal map codec' });
    expect(within(normal).getByRole('button', { name: 'ETC1S' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(screen.getByText('2 maps, 772 KB ETC1S')).toBeInTheDocument();
    expect(screen.getByText('1 map, 412 KB UASTC')).toBeInTheDocument();
    expect(screen.getByText('none in this model')).toBeInTheDocument();
  });

  it('marks a group whose built maps a run would change', () => {
    const model = {
      ...character,
      preset: { ...character.preset, codecs: { ...character.preset.codecs, normal: 'etc1s' } },
    } satisfies InspectModel;
    render(<MapCodecRows model={model} onCodec={vi.fn()} />);
    expect(screen.getByText('1 map, 412 KB UASTC')).toHaveAttribute(
      'title',
      'Built as UASTC. Compress encodes them as ETC1S on the next run.'
    );
  });

  it('saves a group codec, and does nothing while the row is locked', async () => {
    const onCodec = vi.fn();
    const { unmount } = render(<MapCodecRows model={character} onCodec={onCodec} />);
    const normal = screen.getByRole('group', { name: 'Normal map codec' });
    await userEvent.click(within(normal).getByRole('button', { name: 'UASTC' }));
    expect(onCodec).toHaveBeenCalledWith('normal', 'uastc');
    unmount();

    render(<MapCodecRows model={character} onCodec={null} />);
    expect(screen.getByRole('group', { name: 'Color map codec' })).toHaveClass(
      'pointer-events-none'
    );
  });
});
