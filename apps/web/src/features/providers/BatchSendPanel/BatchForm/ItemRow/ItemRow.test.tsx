import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { BatchItemDraft } from '@/state/batch';
import { ItemRow } from '.';

const item: BatchItemDraft = {
  asset: 'lantern-post',
  name: 'Lantern post',
  variant: 'prop',
  images: ['refs/lamp_front.png', 'refs/lamp_side.png'],
  thumbnail: null,
  tripo: {},
  meshy: {},
  studio: {},
};

function row(props: Partial<Parameters<typeof ItemRow>[0]> = {}) {
  const handlers = { onVariant: vi.fn(), onToggle: vi.fn(), onClear: vi.fn() };
  render(
    <ul>
      <ItemRow
        item={item}
        projectDir="/p"
        diff={null}
        open={false}
        fields={<p>fields</p>}
        {...handlers}
        {...props}
      />
    </ul>
  );
  return handlers;
}

describe('a batch item row', () => {
  it('names the refs and changes the kind', async () => {
    const { onVariant } = row();
    expect(screen.getByText('2 images: lamp_front.png, lamp_side.png')).toBeInTheDocument();
    await userEvent.click(
      within(screen.getByRole('group', { name: 'Lantern post kind' })).getByRole('button', {
        name: 'Character',
      })
    );
    expect(onVariant).toHaveBeenCalledWith('character');
  });

  it('marks a custom asset and clears it from its open settings', async () => {
    const { onClear } = row({ diff: 'Faces 20k', open: true });
    expect(screen.getByText('Custom')).toBeInTheDocument();
    expect(screen.getByText('Faces 20k')).toBeInTheDocument();
    expect(screen.getByText('fields')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Use batch settings' }));
    expect(onClear).toHaveBeenCalledOnce();
  });

  it('warns when the asset has no refs', () => {
    row({ item: { ...item, images: [] } });
    expect(screen.getByText('No refs yet')).toBeInTheDocument();
  });
});
