import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CanvasBoundary from './CanvasBoundary';

let broken = true;
function Canvas() {
  if (broken) throw new Error('Invalid hook call');
  return <div>canvas drawn</div>;
}

beforeEach(() => {
  broken = true;
  vi.spyOn(console, 'error').mockImplementation(() => {});
});
afterEach(() => vi.restoreAllMocks());

describe('the canvas boundary', () => {
  it('shows a message with Reload instead of a blank window, and tries again on Reload', async () => {
    render(
      <CanvasBoundary resetKey="a">
        <Canvas />
      </CanvasBoundary>
    );
    expect(screen.getByRole('alert')).toHaveTextContent('The 3D preview stopped');
    expect(screen.getByText('Invalid hook call')).toBeInTheDocument();
    broken = false;
    await userEvent.click(screen.getByRole('button', { name: 'Reload' }));
    expect(screen.getByText('canvas drawn')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('tries again by itself when the model changes', () => {
    const tree = (key: string) => (
      <CanvasBoundary resetKey={key}>
        <Canvas />
      </CanvasBoundary>
    );
    const { rerender } = render(tree('a'));
    expect(screen.getByRole('alert')).toBeInTheDocument();
    broken = false;
    rerender(tree('b'));
    expect(screen.getByText('canvas drawn')).toBeInTheDocument();
  });
});
