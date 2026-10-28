import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ROW, useList } from './windowed-list.hooks';

interface ListProps {
  count: number;
  columns?: number;
  onRender?: () => void;
}

function List({ count, columns, onRender }: ListProps) {
  const { scrollRef, w } = useList(count, columns);
  onRender?.();
  const items = [];
  for (let i = w.start; i < w.end; i++) {
    items.push(
      <li key={i} {...w.itemProps(i)}>
        item {i}
      </li>
    );
  }
  return (
    <div ref={scrollRef} data-testid="scroller" style={{ overflowY: 'auto' }}>
      <ul data-testid="list" style={{ paddingTop: w.before, paddingBottom: w.after }}>
        {items}
      </ul>
    </div>
  );
}

const shown = () => screen.queryAllByRole('listitem').map(li => li.textContent);

describe('a windowed list', () => {
  it('renders a short list whole', () => {
    render(<List count={30} />);
    expect(shown()).toHaveLength(30);
    expect(screen.getByTestId('list').style.paddingBottom).toBe('0px');
  });

  it('renders the rows in view of a long list and keeps the space of the rest', () => {
    render(<List count={500} />);
    const n = shown().length;
    expect(n).toBeGreaterThan(10);
    expect(n).toBeLessThan(40);
    expect(shown()[0]).toBe('item 0');
    expect(screen.getByTestId('list').style.paddingBottom).toBe(`${(500 - n) * ROW}px`);
  });

  it('moves the window as the list scrolls', () => {
    render(<List count={500} />);
    const scroller = screen.getByTestId('scroller');
    act(() => {
      scroller.scrollTop = 200 * ROW;
      fireEvent.scroll(scroller);
    });
    expect(shown()).toContain('item 205');
    expect(shown()).not.toContain('item 0');
    const before = parseInt(screen.getByTestId('list').style.paddingTop, 10);
    expect(before).toBeGreaterThan(190 * ROW);
  });

  it('windows a grid by rows, measuring the first item of each row', () => {
    render(<List count={500} columns={4} />);
    const items = screen.getAllByRole('listitem');
    expect(items.length % 4).toBe(0);
    expect(items[0]).toHaveAttribute('data-index', '0');
    expect(items[1]).not.toHaveAttribute('data-index');
    expect(items[4]).toHaveAttribute('data-index', '1');
  });

  it('renders again only when the window moves, not on every scroll event', () => {
    let renders = 0;
    render(<List count={500} onRender={() => renders++} />);
    const scroller = screen.getByTestId('scroller');
    const settled = renders;
    act(() => {
      scroller.scrollTop = 1;
      fireEvent.scroll(scroller);
    });
    expect(renders).toBe(settled);
    act(() => {
      scroller.scrollTop = 100 * ROW;
      fireEvent.scroll(scroller);
    });
    expect(renders).toBeGreaterThan(settled);
  });
});
