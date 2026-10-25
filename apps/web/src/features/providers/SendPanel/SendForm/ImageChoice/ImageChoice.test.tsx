import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ImageChoice } from '.';

const image = { file: 'refs/lamp_side.png', name: 'lamp_side.png', bytes: 2048 };

function choice(order: number, full = false) {
  const onToggle = vi.fn();
  const onFront = vi.fn();
  render(
    <ul>
      <ImageChoice
        image={image}
        projectDir="/p"
        order={order}
        full={full}
        onToggle={onToggle}
        onFront={onFront}
      />
    </ul>
  );
  return { onToggle, onFront };
}

describe('an image choice', () => {
  it('toggles an unpicked image, unless the send is full', async () => {
    const { onToggle } = choice(-1);
    const button = screen.getByRole('button', { pressed: false });
    expect(button).toHaveAttribute('title', 'lamp_side.png, 2 KB');
    await userEvent.click(button);
    expect(onToggle).toHaveBeenCalledOnce();
  });

  it('is disabled when the send is full and it is not picked', () => {
    choice(-1, true);
    expect(screen.getByRole('button', { pressed: false })).toBeDisabled();
  });

  it('numbers a picked image and makes it the front', async () => {
    const { onFront } = choice(1);
    expect(screen.getByRole('button', { pressed: true })).toHaveTextContent(/^2 /);
    await userEvent.click(screen.getByRole('button', { name: 'Make front' }));
    expect(onFront).toHaveBeenCalledOnce();
  });
});
