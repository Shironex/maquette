import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { Spend } from '@/ipc/client';
import { SpendRow } from '.';

const NOW = 1_800_000_000_000;

const spend: Spend = {
  project: 'frostveil',
  job: 'veil-gate-3',
  asset: 'veil-gate',
  assetName: 'Veil gate',
  service: 'meshy',
  task: 'task-1',
  summary: 'Meshy 6, 1 image, 4K PBR',
  sentAtMs: NOW - 2 * 3_600_000,
  settledAtMs: null,
  credits: 30,
  estimated: true,
  outcome: 'pending',
  redoOf: 'veil-gate-2',
  viaAgent: true,
  source: 'api',
};

describe('a spend row', () => {
  it('shows a running redo with its estimated credits', () => {
    render(
      <ul>
        <SpendRow spend={spend} now={NOW} project="Frostveil" />
      </ul>
    );
    const row = screen.getByRole('listitem');
    expect(row).toHaveTextContent('2 h ago');
    expect(row).toHaveTextContent('Frostveil · Meshy · by an agent');
    expect(screen.getByTitle('Redo of veil-gate-2')).toHaveTextContent('Redo');
    expect(row).toHaveTextContent('Running');
    expect(row).toHaveTextContent('est.30');
  });
});
