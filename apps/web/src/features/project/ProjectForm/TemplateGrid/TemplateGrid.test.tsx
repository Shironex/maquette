import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TemplateGrid } from '.';

afterEach(cleanup);

describe('the template grid', () => {
  it('marks the chosen template and picks another', async () => {
    const onChange = vi.fn();
    render(<TemplateGrid value="room" onChange={onChange} />);
    expect(screen.getByRole('button', { name: /Room/ })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('Selected')).toBeVisible();
    expect(
      screen.getByText('Creates one website scene: an interior with baked light')
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: /Floating diorama/ }));
    expect(onChange).toHaveBeenCalledWith('floating-diorama');
  });

  it('drops the details when compact', () => {
    render(<TemplateGrid value="room" compact onChange={vi.fn()} />);
    expect(
      screen.queryByText('Creates one website scene: an interior with baked light')
    ).toBeNull();
  });
});
