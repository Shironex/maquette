import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { PartRow } from '@/ipc/client';
import { PartActions, PartsList } from '.';

const row = (id: string, name: string, extra: Partial<PartRow> = {}): PartRow => ({
  id,
  name,
  triangles: 2_400,
  materials: ['stone'],
  hidden: false,
  joined: [],
  ...extra,
});

describe('PartsList', () => {
  it('keeps the old name when a rename is cancelled', async () => {
    const onRename = vi.fn();
    render(
      <PartsList
        rows={[row('part_1', 'lid')]}
        selected={[]}
        onPick={vi.fn()}
        onRename={onRename}
        onUnmerge={vi.fn()}
      />
    );
    await userEvent.dblClick(screen.getByRole('button', { name: 'lid' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Rename lid' }), 'x{Escape}');
    expect(onRename).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'lid' })).toBeInTheDocument();
  });

  it('lists a join under its first part and undoes it', async () => {
    const onUnmerge = vi.fn();
    render(
      <PartsList
        rows={[row('part_4', 'post', { joined: ['part_5'], triangles: 5_700 })]}
        selected={['part_4']}
        onPick={vi.fn()}
        onRename={vi.fn()}
        onUnmerge={onUnmerge}
      />
    );
    expect(screen.getByRole('button', { name: /^post\s*\+ part_5$/ })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(screen.getByText('5.7k')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Undo the join into post' }));
    expect(onUnmerge).toHaveBeenCalledWith('part_4');
  });
});

describe('PartActions', () => {
  it('merges two or more and hides any', () => {
    render(<PartActions count={1} allHidden={false} onMerge={vi.fn()} onHide={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Merge' })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('button', { name: 'Hide' })).not.toHaveAttribute('aria-disabled');
  });
});
