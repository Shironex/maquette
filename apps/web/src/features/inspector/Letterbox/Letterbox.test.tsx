import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import Letterbox from './Letterbox';

describe('the letterbox', () => {
  it('frames 16:9 inside the view and lets clicks through', () => {
    const { container } = render(<Letterbox />);
    const root = container.firstElementChild!;
    expect(root).toHaveClass('pointer-events-none', 'absolute', 'inset-0');
    expect(root.firstElementChild).toHaveClass('aspect-video');
    expect(root.firstElementChild).toHaveStyle({ width: 'min(100cqw, calc(100cqh * 16 / 9))' });
  });
});
