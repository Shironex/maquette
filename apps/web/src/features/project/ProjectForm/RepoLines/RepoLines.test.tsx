import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { RepoLines } from '.';

describe('RepoLines', () => {
  it('lists each line with its detail', () => {
    render(
      <RepoLines
        label="Game repo check"
        lines={[
          { tone: 'found', text: 'Git repo found.', detail: 'On main.' },
          { tone: 'info', text: 'Ship writes to assets/models/.' },
        ]}
      />
    );
    const list = screen.getByRole('list', { name: 'Game repo check' });
    expect(list.children).toHaveLength(2);
    expect(screen.getByText('On main.')).toBeInTheDocument();
  });

  it('renders nothing without lines', () => {
    const { container } = render(<RepoLines label="Check" lines={[]} />);
    expect(container).toBeEmptyDOMElement();
  });
});
