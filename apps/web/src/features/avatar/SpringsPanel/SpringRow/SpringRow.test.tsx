import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { SpringCandidate } from '@/ipc/client';
import SpringRow from './SpringRow';

const CANDIDATE: SpringCandidate = {
  id: 'cloth-2',
  kind: 'cloth',
  rootBone: 'hips',
  line: [
    [0, 1, -0.1],
    [0, 0.5, -0.2],
  ],
  length: 0.48,
  drop: 0.5,
  vertices: 1015,
  confidence: 0.45,
  joints: 4,
  color: '#e0a040',
};

function row(spring: Parameters<typeof SpringRow>[0]['spring']) {
  const onConfirm = vi.fn();
  const onEdit = vi.fn();
  render(
    <ul>
      <SpringRow
        candidate={CANDIDATE}
        spring={spring}
        disabled={false}
        onConfirm={onConfirm}
        onEdit={onEdit}
      />
    </ul>
  );
  return { onConfirm, onEdit, item: screen.getByRole('listitem', { name: /cloth-2/ }) };
}

describe('a part that could swing', () => {
  it('says what it is and asks before it swings', async () => {
    const { onConfirm, item } = row(null);
    expect(within(item).getByText(/1,015 vertices/)).toBeInTheDocument();
    expect(within(item).queryByRole('combobox')).toBeNull();
    await userEvent.click(within(item).getByRole('switch', { name: 'Swing cloth-2' }));
    expect(onConfirm).toHaveBeenCalledWith(true);
  });

  it('once confirmed, swings heavy by its kind until another preset is picked', async () => {
    const { onEdit, item } = row({ ...CANDIDATE, joints: 4 });
    expect(within(item).getByText('More drag and gravity: cloth')).toBeInTheDocument();
    await userEvent.selectOptions(
      within(item).getByRole('combobox', { name: 'How cloth-2 swings' }),
      'stiff'
    );
    expect(onEdit).toHaveBeenCalledWith({ preset: 'stiff' });
  });
});
