import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { AssetRef } from '@/ipc/client';
import { ViewTile } from '.';

const refs: AssetRef[] = [
  { file: 'refs/lantern.png', view: 'front' },
  { file: 'refs/lantern_side.png', view: 'left' },
];

function tile(item: AssetRef) {
  const actions = { onTag: vi.fn(), onClean: vi.fn(), onRemove: vi.fn() };
  render(
    <ul>
      <ViewTile item={item} projectDir="/p" refs={refs} {...actions} />
    </ul>
  );
  return actions;
}

describe('a ref tile', () => {
  it('names its view on the chip and changes it from the menu', async () => {
    const { onTag } = tile(refs[0]!);
    await userEvent.click(screen.getByRole('button', { name: 'View of lantern.png: Front' }));
    expect(screen.getByRole('menuitem', { name: 'Front (now)' })).toBeDisabled();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Left' }));
    expect(onTag).toHaveBeenCalledWith('left');
  });

  it('offers Clean background and Leave out', async () => {
    const { onClean, onRemove } = tile(refs[1]!);
    await userEvent.click(screen.getByRole('button', { name: 'View of lantern_side.png: Left' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Clean background...' }));
    expect(onClean).toHaveBeenCalledOnce();
    await userEvent.click(screen.getByRole('button', { name: 'View of lantern_side.png: Left' }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Leave out of the refs' }));
    expect(onRemove).toHaveBeenCalledOnce();
  });
});
