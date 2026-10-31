import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { RestCompare } from '@/ipc/client';
import RestOverlay from './RestOverlay';

const bone = (name: string, y: number) => ({
  name,
  parent: null,
  head: [0, y, 0] as [number, number, number],
  tail: [0, y + 0.1, 0] as [number, number, number],
});

const rest: RestCompare = {
  clip: [bone('Hips', 1)],
  model: [bone('hips', 0.94)],
  clipShape: 't-pose',
  modelShape: 'a-pose',
  clipArmsDeg: 0,
  modelArmsDeg: 47,
  clipHips: 1.04,
  modelHips: 0.98,
  hipsScale: 0.94,
  turns: [],
};

describe('the rest pose overlay', () => {
  it('says how the two stand and shows them together or apart', () => {
    render(<RestOverlay rest={rest} clipLabel="Mixamo" modelLabel="Elsheria" />);
    expect(screen.getByText('47° below the T pose')).toBeInTheDocument();
    expect(screen.getByText('0.98 m, Mixamo 1.04 m')).toBeInTheDocument();
    expect(screen.getByText('0.94')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Mixamo T pose over Elsheria A pose' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Side by side' }));
    expect(screen.getByRole('img', { name: 'Elsheria A pose' })).toBeVisible();
    expect(screen.getByRole('img', { name: 'Mixamo T pose' })).toBeVisible();
  });
});
