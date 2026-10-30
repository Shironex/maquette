import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import ReadinessPanel from './ReadinessPanel';

afterEach(() => clearMocks());

describe('the readiness panel', () => {
  it('lists every finding, blockers first, the arm weights among them', async () => {
    await openAvatar();
    render(<ReadinessPanel />);
    const panel = screen.getByRole('region', { name: 'VRM readiness' });
    expect(panel).toHaveTextContent('2 blockers. A build fixes the rest pose first');
    const titles = [
      'Faces +X, VRM wants +Z',
      'Arms hang below horizontal, VRM wants a T-pose',
      '212 vertices of cloth or hair weighted to the arms',
      '52 of 55 humanoid bones mapped',
    ];
    const text = panel.textContent ?? '';
    const at = titles.map(t => text.indexOf(t));
    expect(at.every(i => i >= 0)).toBe(true);
    expect([...at].sort((a, b) => a - b)).toEqual(at);
  });
});
