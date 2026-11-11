import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { RecipeReviewFix } from '@/ipc/client';
import RecipeFixes from './RecipeFixes';

const FIXES: RecipeReviewFix[] = [
  {
    id: 'blend-1',
    line: 'seam',
    title: 'Widen the blend to 12 f',
    detail: 'Spreads the hip move over more frames.',
    after: 'about 1.2 cm',
    passes: true,
    edit: { kind: 'blend', clip: 1, frames: 12 },
  },
  {
    id: 'ease-0-20-in',
    line: 'hold-seam',
    title: 'Ease the hold in over 12 f',
    detail: 'Spreads the hip move over more frames.',
    after: 'about 2.4 cm',
    passes: false,
    edit: { kind: 'hold-ease', clip: 0, at: 20, easeIn: 12, easeOut: null },
  },
];

describe('the review fixes', () => {
  it('lists each fix in order with what it comes to and tries the one picked', async () => {
    const onTry = vi.fn();
    render(<RecipeFixes fixes={FIXES} onTry={onTry} />);
    const list = within(screen.getByRole('region', { name: 'Fixes' })).getAllByRole('listitem');
    expect(list[0]).toHaveTextContent('1Widen the blend to 12 f');
    expect(list[0]).toHaveTextContent('after: about 1.2 cm, passes');
    expect(list[1]).toHaveTextContent('after: about 2.4 cm, still over');
    await userEvent.click(within(list[1]!).getByRole('button', { name: 'Try it' }));
    expect(onTry).toHaveBeenCalledWith(FIXES[1]);
  });

  it('shows nothing without fixes, and why a try laid nothing', () => {
    const { container, rerender } = render(<RecipeFixes fixes={[]} onTry={() => {}} />);
    expect(container).toBeEmptyDOMElement();
    rerender(<RecipeFixes fixes={FIXES} onTry={() => {}} error="The recipe no longer has it." />);
    expect(screen.getByText('The recipe no longer has it.')).toBeVisible();
  });
});
