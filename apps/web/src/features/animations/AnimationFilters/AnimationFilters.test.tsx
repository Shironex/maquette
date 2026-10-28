import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import { NO_ANIMATION_FILTERS } from '@/state/animations';
import { AnimationFilters } from '.';

const items = animationClips().map(clip => ({ clip, file: `/lib/${clip.id}`, uses: [] }));

describe('the animation filters', () => {
  it('counts packs, tags and rigs over the whole library and toggles them', async () => {
    const onChange = vi.fn();
    render(
      <AnimationFilters
        items={items}
        filters={{ ...NO_ANIMATION_FILTERS, pack: 'Singles' }}
        onChange={onChange}
        dir="~/Maquette/library/animations"
      />
    );
    const packs = screen.getByRole('group', { name: 'Packs' });
    expect(within(packs).getByRole('button', { name: /All clips/ })).toHaveTextContent('16');
    await userEvent.click(within(packs).getByRole('button', { name: /Singles/ }));
    expect(onChange).toHaveBeenLastCalledWith({ pack: null });
    const rigs = screen.getByRole('group', { name: 'Source rig' });
    await userEvent.click(within(rigs).getByRole('button', { name: /Custom/ }));
    expect(onChange).toHaveBeenLastCalledWith({ rig: 'custom' });
    expect(screen.getByRole('group', { name: 'Tags' })).toHaveTextContent('dodge');
    expect(screen.getByText(/library\/animations/)).toBeVisible();
  });
});
