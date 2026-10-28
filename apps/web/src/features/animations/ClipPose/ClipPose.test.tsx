import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { restBones } from '@/ipc/mock/animation-fixtures';
import { ClipPose } from '.';

describe('the clip pose', () => {
  it('draws a line and a joint for every bone', () => {
    const { container } = render(<ClipPose bones={restBones('mixamo')} />);
    expect(container.querySelectorAll('circle')).toHaveLength(22);
    // The floor line, then one per bone.
    expect(container.querySelectorAll('line')).toHaveLength(23);
  });

  it('draws only the floor for a clip without bones', () => {
    const { container } = render(<ClipPose bones={[]} />);
    expect(container.querySelectorAll('line')).toHaveLength(1);
  });
});
