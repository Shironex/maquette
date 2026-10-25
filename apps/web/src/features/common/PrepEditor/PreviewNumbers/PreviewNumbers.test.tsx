import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { FixupReport } from '@/ipc/client';
import { PreviewNumbers } from '.';

const numbers = (triangles: number, objects: number, maxTexture: number, h: number) => ({
  triangles,
  objects,
  maxTexture,
  min: [-0.25, 0, -0.25] as [number, number, number],
  max: [0.25, h, 0.25] as [number, number, number],
});

const report: FixupReport = {
  plan: { size: null, fixups: [{ op: 'decimate', triangles: 5000 }] },
  before: numbers(19_224, 3, 1024, 0.7),
  after: numbers(4999, 1, 256, 0.45),
  steps: [
    {
      op: 'decimate',
      before: numbers(19_212, 1, 1024, 0.7),
      after: numbers(4999, 1, 1024, 0.7),
      note: 'ratio 0.260',
    },
  ],
  scale: 0.643,
  bytesBefore: 9_999_392,
  bytesAfter: 125_556,
  output: '/cache/fixups/abc/fixed.glb',
};

describe('PreviewNumbers', () => {
  it('shows each number before and after, then what each fix-up did', () => {
    render(<PreviewNumbers report={report} />);
    const tris = screen.getByRole('row', { name: /Triangles/ });
    expect(within(tris).getByText('19,224')).toBeInTheDocument();
    expect(within(tris).getByText('4,999')).toHaveClass('text-sage');
    expect(screen.getByRole('row', { name: /File/ })).toHaveTextContent('10.0 MB');
    expect(screen.getByRole('row', { name: /Size/ })).toHaveTextContent('50×45×50 cm');
    expect(screen.getByText('19,212 to 4,999 triangles, ratio 0.260')).toBeInTheDocument();
  });
});
