import { describe, expect, it } from 'vitest';
import { FINGER_SLACK_DEG, fingersSummary } from './avatar-fingers';

describe('the fingers summary', () => {
  it('says kept, straightened, or how far each hand went', () => {
    expect(fingersSummary({ fingers: 'keep' })).toBe('fingers kept');
    expect(fingersSummary({ fingers: 'straighten', straightened: { left: { share: 1 } } })).toBe(
      'fingers straightened'
    );
    expect(
      fingersSummary({
        fingers: 'straighten',
        straightened: {
          left: { share: 0.25, stretch: 1.18 },
          right: { share: 0, skipped: 'the finger bones sit bunched at the wrist' },
        },
      })
    ).toBe('fingers straightened (left hand 25%, right hand left as it is)');
  });

  it('counts a soft curl as straight, as the readiness does', () => {
    expect(FINGER_SLACK_DEG).toBe(15);
  });
});
