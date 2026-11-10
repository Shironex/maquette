import { describe, expect, it } from 'vitest';
import { exportName, namePart, nextJobId } from './export-name';

const job = (project: string, asset: string, n: number, service: 'tripo' | 'meshy' = 'tripo') => ({
  id: `${asset}-${n}`,
  project,
  asset,
  service,
});

describe('exportName', () => {
  it('carries the project, the asset, the send number and the provider', () => {
    expect(exportName(job('moon-island', 'crate-lid', 3))).toBe(
      'maq_moon-island_crate-lid_v3_tripo'
    );
    expect(exportName(job('studio', 'lamp', 1, 'meshy'))).toBe('maq_studio_lamp_v1_meshy');
  });

  it('gives two projects sending the same asset two names', () => {
    const first = exportName(job('moon-island', 'desk-lamp', 1));
    const second = exportName(job('lantern-bay', 'desk-lamp', 1));
    expect(first).toBe('maq_moon-island_desk-lamp_v1_tripo');
    expect(second).toBe('maq_lantern-bay_desk-lamp_v1_tripo');
  });

  it('cuts a long or odd id the way the core does', () => {
    // The same strings the core's naming tests pin.
    expect(namePart('moon-island', 24, 'project')).toBe('moon-island');
    expect(namePart('the-very-long-name-of-a-project-alpha', 24, 'project')).toBe(
      'the-very-long-na--8ecd3d'
    );
    expect(namePart('Moon_Island v2', 24, 'project')).toMatch(/^moon-island-v2--[0-9a-f]{6}$/);
    expect(namePart('a'.repeat(24), 24, 'project')).toBe('a'.repeat(24));
    expect(namePart('a'.repeat(25), 24, 'project')).toHaveLength(24);
    expect(namePart('雪', 24, 'project')).toMatch(/^project--[0-9a-f]{6}$/);
  });
});

describe('nextJobId', () => {
  it('numbers an asset in its own project, one past its highest', () => {
    const jobs = [
      { id: 'desk-lamp-1', project: 'moon' },
      { id: 'desk-lamp-3', project: 'moon' },
      { id: 'desk-lamp-9', project: 'bay' },
      { id: 'desk-lamp-2-1', project: 'moon' },
    ];
    expect(nextJobId(jobs, 'moon', 'desk-lamp')).toBe('desk-lamp-4');
    expect(nextJobId(jobs, 'bay', 'desk-lamp')).toBe('desk-lamp-10');
    expect(nextJobId(jobs, 'cove', 'desk-lamp')).toBe('desk-lamp-1');
    expect(nextJobId(jobs, 'moon', 'desk-lamp-2')).toBe('desk-lamp-2-2');
  });
});
