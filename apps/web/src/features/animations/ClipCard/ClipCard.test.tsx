import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import { ClipCard } from '.';

const clip = (name: string) => {
  const found = animationClips().find(c => c.name === name)!;
  return { clip: found, file: `/lib/${found.id}/clip.glb` };
};

describe('the clip card', () => {
  it('shows the rig, the length, root motion and a loop', async () => {
    const onSelect = vi.fn();
    render(<ClipCard item={clip('Sword And Shield Run')} selected={false} onSelect={onSelect} />);
    const card = screen.getByRole('button', { name: 'Sword And Shield Run' });
    expect(card).toHaveTextContent('Mixamo');
    expect(card).toHaveTextContent('22 f · 0.73 s');
    expect(card).toHaveTextContent('root');
    expect(card).toHaveTextContent('loop');
    await userEvent.click(card);
    expect(onSelect).toHaveBeenCalled();
  });

  it('marks a clip that stays put', () => {
    render(<ClipCard item={clip('Sword And Shield Slash')} selected onSelect={vi.fn()} />);
    const card = screen.getByRole('button', { pressed: true });
    expect(card).toHaveTextContent('in place');
    expect(card).not.toHaveTextContent('loop');
  });
});
