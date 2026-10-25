import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { styledLine } from './styledLine';

const row = (text: string) => ({ t: '12:00', text, level: 'info' as const, step: null });

describe('styledLine', () => {
  it('dims the tools chatter and marks it as noise', () => {
    expect(styledLine(row('Blender 4.2.0'))).toMatchObject({ dim: true, noise: true });
  });

  it('keeps the tag of a script line in clay and the final size in bold', () => {
    const line = styledLine(row('[compress] island.glb 4.6 MB → 1.1 MB'));
    expect(line.noise).toBe(false);
    const { container } = render(<>{line.node}</>);
    expect(container.querySelector('.text-clay-text')).toHaveTextContent('[compress]');
    expect(container.querySelector('strong')).toHaveTextContent('1.1 MB');
  });

  it('takes the error level for a failure', () => {
    expect(styledLine(row('blender exited with code 1')).level).toBe('error');
  });
});
