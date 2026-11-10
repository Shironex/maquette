import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import BoneMap from './BoneMap';

describe('the bone picker', () => {
  it('marks the keyed bones, lists them with their counts, and picks from the map or the list', async () => {
    const onPick = vi.fn();
    render(
      <BoneMap
        bone="chest"
        counts={
          new Map([
            ['chest', 3],
            ['rightUpperArm', 2],
          ])
        }
        onPick={onPick}
      />
    );
    expect(screen.getByRole('button', { name: 'Chest, 3 keys' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(screen.getByRole('button', { name: 'Left upper arm' })).toBeVisible();
    expect(screen.getByRole('button', { name: /Right upper arm\s*2 keys/ })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Head' }));
    expect(onPick).toHaveBeenLastCalledWith('head');
    await userEvent.selectOptions(screen.getByLabelText('Any bone'), 'leftIndexProximal');
    expect(onPick).toHaveBeenLastCalledWith('leftIndexProximal');
  });
});
