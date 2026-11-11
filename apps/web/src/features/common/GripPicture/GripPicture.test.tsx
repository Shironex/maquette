import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import GripPicture from './GripPicture';
import { handPicture } from './hand-picture';

const curled = (side: 'left' | 'right', curl: number) =>
  Object.fromEntries(
    ['Index', 'Middle', 'Ring', 'Little'].flatMap(f =>
      ['Proximal', 'Intermediate', 'Distal'].map(j => [`${side}${f}${j}`, [0, 0, curl]])
    )
  );

describe('GripPicture', () => {
  it('draws one hand for a one hand pose and both for a two hand one', () => {
    const { container, rerender } = render(
      <GripPicture pose={curled('right', 40)} label="Right hand" />
    );
    expect(screen.getByRole('img', { name: 'Right hand' })).toBeInTheDocument();
    expect(container.querySelectorAll('g')).toHaveLength(1);
    rerender(
      <GripPicture
        pose={{ ...curled('right', 40), ...curled('left', -40) }}
        holds="right"
        label="Both hands"
      />
    );
    expect(container.querySelectorAll('g')).toHaveLength(2);
    expect(container.querySelector('svg')).toHaveAttribute('viewBox', '0 0 120 80');
  });

  it('shortens a finger the more it curls', () => {
    const tip = (curl: number) =>
      handPicture(curled('right', curl), 'right').fingers[1]!.at(-1)![1];
    // The fingers point up, so a longer finger ends higher (a smaller y).
    expect(tip(0)).toBeLessThan(tip(30));
    expect(tip(30)).toBeLessThan(tip(60));
  });

  it('mirrors the left hand across the box', () => {
    const right = handPicture(curled('right', 20), 'right');
    const left = handPicture(curled('left', -20), 'left');
    expect(left.fingers[1]![0]![0]).toBe(60 - right.fingers[1]![0]![0]);
    expect(left.fingers[1]!.at(-1)![1]).toBe(right.fingers[1]!.at(-1)![1]);
  });
});
