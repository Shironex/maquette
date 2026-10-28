import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { LogConsole, type LogLine } from '../components/content/LogConsole';

const many = (n: number): LogLine[] =>
  Array.from({ length: n }, (_, i) => ({ t: '12:00:00', text: `line ${i}` }));

describe('the log console', () => {
  it('renders every line of a short log', () => {
    render(<LogConsole lines={many(20)} />);
    expect(screen.getByText('line 0')).toBeInTheDocument();
    expect(screen.getByText('line 19')).toBeInTheDocument();
  });

  it('renders only the lines in view of a long log', () => {
    render(<LogConsole lines={many(5000)} />);
    const log = screen.getByRole('log');
    expect(screen.getByText('line 0')).toBeInTheDocument();
    expect(screen.queryByText('line 4999')).toBeNull();
    expect(log.querySelectorAll('[data-index]').length).toBeLessThan(100);
  });

  it('keeps the newest line in view while lines arrive', () => {
    // The test DOM neither lays out nor scrolls: give the log a height and
    // move scrollTop as a browser would.
    const scrollTo = vi.fn(function (this: HTMLElement, to: ScrollToOptions) {
      this.scrollTop = to.top ?? this.scrollTop;
      this.dispatchEvent(new Event('scroll'));
    });
    const proto = HTMLElement.prototype;
    Object.defineProperty(proto, 'scrollTo', { value: scrollTo, configurable: true });
    Object.defineProperty(proto, 'scrollHeight', { get: () => 1e6, configurable: true });
    const { rerender } = render(<LogConsole lines={many(5000)} autoScroll />);
    rerender(<LogConsole lines={many(5001)} autoScroll />);
    expect(screen.getByText('line 5000')).toBeInTheDocument();
    expect(screen.queryByText('line 0')).toBeNull();
    Reflect.deleteProperty(proto, 'scrollTo');
    Reflect.deleteProperty(proto, 'scrollHeight');
  });
});
