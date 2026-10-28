import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { RefWarning } from '@/ipc/client';
import { FIX_LABEL } from '@/state/refs';
import { RefWarnings } from '.';

const busy: RefWarning = {
  kind: 'busy-background',
  file: 'refs/lantern_kit.png',
  view: 'front',
  title: 'Busy background in lantern_kit.png',
  detail: 'Clutter around the object can end up in the model.',
  fixes: ['clean-background', 'leave-out'],
};

describe('ref warnings', () => {
  it('lists each warning with its one click fixes', async () => {
    const onFix = vi.fn();
    render(<RefWarnings warnings={[busy]} onFix={onFix} />);
    expect(screen.getByRole('list', { name: 'Ref warnings' })).toHaveTextContent(busy.title);
    await userEvent.click(screen.getByRole('button', { name: 'Leave out this image' }));
    expect(onFix).toHaveBeenCalledWith(busy, 'leave-out');
    expect(screen.getByRole('button', { name: 'Clean background' })).toBeInTheDocument();
  });

  it('says a back that does not match the front looks different, with a redraw', async () => {
    const onFix = vi.fn();
    const back: RefWarning = {
      kind: 'views-disagree',
      file: 'refs/lantern_back.png',
      view: 'back',
      title: 'Back looks different from the front: its colours do not match',
      detail: 'Views that do not match can mix into the model.',
      fixes: ['make-views', 'leave-out'],
    };
    render(<RefWarnings warnings={[busy, back]} onFix={onFix} />);
    const list = screen.getByRole('list', { name: 'Ref warnings' });
    expect(list).toHaveTextContent('Back looks different from the front');
    await userEvent.click(screen.getByRole('button', { name: FIX_LABEL['make-views'] }));
    expect(onFix).toHaveBeenCalledWith(back, 'make-views');
  });

  it('shows nothing without warnings', () => {
    const { container } = render(<RefWarnings warnings={[]} onFix={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });
});
