import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import Hairline from './Hairline';

describe('a hairline', () => {
  it('draws a one pixel rule in the line colour', () => {
    const { container } = render(<Hairline />);
    expect(container.firstElementChild).toHaveClass('h-px', 'flex-none', 'bg-line');
    expect(container.firstElementChild).toBeEmptyDOMElement();
  });
});
