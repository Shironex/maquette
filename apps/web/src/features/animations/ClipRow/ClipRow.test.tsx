import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import { ClipRow } from '.';

describe('the clip row', () => {
  it('names the pack and the source rig', () => {
    const clip = animationClips().find(c => c.rig.kind === 'tripo')!;
    render(
      <ul>
        <ClipRow item={{ clip, file: '/lib/clip.glb' }} selected={false} onSelect={vi.fn()} />
      </ul>
    );
    const row = screen.getByRole('button', { name: 'Duelist lunge' });
    expect(row).toHaveTextContent('Tripo animations · Tripo');
    expect(row).toHaveTextContent('36 f · 1.50 s');
  });
});
