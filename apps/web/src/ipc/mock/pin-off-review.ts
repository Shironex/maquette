import type { ClipReview } from '../client';

/** A clip whose pin slides more than it does without: the core left the pin off. */
export function pinOffReview(base: ClipReview, pinned: boolean): ClipReview {
  return {
    ...base,
    status: 'look',
    slideCm: 2.8,
    slideFrames: [3, 12],
    pinWorseCm: pinned ? 8.9 : null,
    note: `The left foot slides 2.8 cm while planted, frames 3 to 12${
      pinned ? '. Pinning the feet slid 8.9 cm, so the pin is off' : ''
    }`,
  };
}
