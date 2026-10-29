import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { RefWarning } from '@/ipc/client';
import { ModelNote } from '.';

const note: RefWarning = {
  kind: 'has-model',
  file: null,
  view: null,
  title: 'Stone well already has a model (v1, from the library entry stone-well)',
  detail: 'This send makes v2; the library entry is not changed. The older versions stay.',
  fixes: [],
};

describe('model note', () => {
  it('says what the send makes and that the library entry stays', () => {
    render(<ModelNote note={note} />);
    const found = screen.getByRole('note', { name: 'Existing model' });
    expect(found).toHaveTextContent('from the library entry stone-well');
    expect(found).toHaveTextContent('This send makes v2');
  });

  it('shows nothing without a note', () => {
    const { container } = render(<ModelNote note={null} />);
    expect(container).toBeEmptyDOMElement();
  });
});
