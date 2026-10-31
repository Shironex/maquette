import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { Asset } from '@/ipc/client';
import RenderProblems from './RenderProblems';

const asset = { id: 'a', name: 'Market stall' } as Asset;
const source = { from: 'project', project: 'p' } as const;

describe('what went wrong with a render', () => {
  it('offers the fix each problem has', () => {
    render(
      <RenderProblems
        asset={asset}
        name={asset.name}
        source={source}
        problems={[
          {
            style: 'turntable',
            bytes: 6_800_000,
            problem: { kind: 'over-target', message: 'Over.', webpBytes: 1_100_000 },
          },
          {
            style: 'blueprint',
            bytes: null,
            problem: { kind: 'out-of-memory', message: 'Out of memory.', webpBytes: null },
          },
        ]}
      />
    );
    expect(screen.getByRole('button', { name: 'Crop to the model' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Save as WebP' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Try at 2048 px' })).toBeVisible();
    expect(
      screen.getByRole('heading', { name: 'Blueprint stopped on Market stall' })
    ).toBeVisible();
  });
});
