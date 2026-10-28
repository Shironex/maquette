import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useGrid } from './windowed-list.hooks';

/** A grid whose list renders only once its items have loaded, as the libraries do. */
function Grid({ loaded }: { loaded: boolean }) {
  const { listRef, columns } = useGrid();
  return (
    <div data-testid="columns" data-columns={columns}>
      {loaded ? <ul ref={listRef} /> : <p>Loading</p>}
    </div>
  );
}

const columns = () => screen.getByTestId('columns').getAttribute('data-columns');

describe('grid columns', () => {
  it('fits columns across the list', () => {
    render(<Grid loaded />);
    expect(columns()).toBe('4');
  });

  it('fits columns across a list that renders after its data loads', () => {
    const { rerender } = render(<Grid loaded={false} />);
    expect(columns()).toBe('1');
    rerender(<Grid loaded />);
    expect(columns()).toBe('4');
  });
});
