import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AssembleStageProps } from '@/features/viewer/AssembleStage';
import type { DraftPart } from '@/state/split';
import { clearMocks, openMockParts } from '@/test/mock-ipc';
import AssembleViewer from './AssembleViewer';

/** The 3D stage, as what it was given and a broken load. */
vi.mock('@/features/viewer/AssembleStage', () => ({
  default: ({ selected, mode, snap, onSelect, onError }: AssembleStageProps) => (
    <div>
      <p>{`${selected} ${mode} ${snap.move ?? 'free'} ${snap.turn ?? 'free'}`}</p>
      <button type="button" onClick={() => onSelect('sigil-gate-steps')}>
        Click the steps
      </button>
      <button type="button" onClick={() => onError('1 of 2 parts did not load.')}>
        Break
      </button>
    </div>
  ),
}));

const draft = (asset: string, name: string): DraftPart => ({
  asset,
  version: null,
  name,
  pivot: 'bottom-centre',
  at: null,
  turn: 0,
  triangles: 0,
});

beforeEach(async () => {
  await openMockParts();
});
afterEach(() => clearMocks());

describe('AssembleViewer', () => {
  it('puts the gizmo on the part picked or clicked, with snapping for its mode', async () => {
    const parts = [draft('sigil-gate-frame', 'frame'), draft('sigil-gate-steps', 'steps')];
    render(<AssembleViewer parts={parts} onPlace={vi.fn()} />);
    expect(await screen.findByText('sigil-gate-frame translate 0.05 15')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Click the steps' }));
    expect(screen.getByText('sigil-gate-steps translate 0.05 15')).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Move snapping' }), 'Free');
    await userEvent.click(screen.getByRole('button', { name: 'Turn' }));
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Turn snapping' }),
      'Snap 45°'
    );
    expect(screen.getByText('sigil-gate-steps rotate free 45')).toBeInTheDocument();
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Part the gizmo moves' }),
      'frame'
    );
    expect(screen.getByText('sigil-gate-frame rotate free 45')).toBeInTheDocument();
  });

  it('says when a part did not load', async () => {
    render(<AssembleViewer parts={[draft('sigil-gate-frame', 'frame')]} onPlace={vi.fn()} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Break' }));
    expect(screen.getByText('1 of 2 parts did not load.')).toHaveClass('text-amber');
  });
});
