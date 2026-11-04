import { render, renderHook, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useClearanceStore, useFloatingLift } from '@/lib/useFloatingClearance';
import { Sheet, SheetFooter } from '.';

afterEach(() => {
  vi.restoreAllMocks();
  useClearanceStore.setState({ bars: {} });
});

/** Lays every element out `fromBottom` px above the window's bottom edge. */
function layOut(fromBottom: number) {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
    top: window.innerHeight - fromBottom,
    height: 56,
  } as DOMRect);
}

function open(dirty: boolean) {
  const onClose = vi.fn();
  render(
    <Sheet
      id="test"
      icon="cube"
      title="Send Lamp to 3D"
      meta="prop"
      onClose={onClose}
      dirty={dirty}
    >
      <p>Body</p>
    </Sheet>
  );
  return onClose;
}

describe('the sheet', () => {
  it('is a labelled dialog that closes from its button', async () => {
    const onClose = open(false);
    const sheet = screen.getByRole('dialog', { name: 'Send Lamp to 3D' });
    expect(within(sheet).getByText('prop')).toBeInTheDocument();
    // Its title row is no second banner landmark next to the app's own.
    expect(within(sheet).queryByRole('banner')).toBeNull();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Close' }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('asks before closing with unsent changes', async () => {
    const onClose = open(true);
    await userEvent.keyboard('{Escape}');
    const ask = screen.getByRole('dialog', { name: 'Discard changes?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Discard' }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('keeps Tab inside and gives focus back to its opener on close', async () => {
    const onClose = vi.fn();
    const view = (open: boolean) => (
      <>
        <button type="button">Send to 3D</button>
        {open ? (
          <Sheet id="t" icon="cube" title="Send Lamp to 3D" onClose={onClose}>
            <button type="button">Send</button>
          </Sheet>
        ) : null}
      </>
    );
    const { rerender } = render(view(false));
    const opener = screen.getByRole('button', { name: 'Send to 3D' });
    opener.focus();
    rerender(view(true));
    const sheet = screen.getByRole('dialog', { name: 'Send Lamp to 3D' });
    expect(within(sheet).getByRole('button', { name: 'Close' })).toHaveFocus();
    await userEvent.tab();
    expect(within(sheet).getByRole('button', { name: 'Send' })).toHaveFocus();
    await userEvent.tab();
    expect(within(sheet).getByRole('button', { name: 'Close' })).toHaveFocus();
    await userEvent.tab({ shift: true });
    expect(within(sheet).getByRole('button', { name: 'Send' })).toHaveFocus();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
    rerender(view(false));
    expect(opener).toHaveFocus();
  });

  it('puts its footer under the body and keeps the floating cards above it', () => {
    layOut(57);
    render(
      <Sheet
        id="apply"
        icon="cube"
        title="Apply the camera plan"
        onClose={vi.fn()}
        footer={<button type="button">Apply</button>}
      >
        <p>Body</p>
      </Sheet>
    );
    const sheet = screen.getByRole('dialog', { name: 'Apply the camera plan' });
    const footer = within(sheet).getByRole('button', { name: 'Apply' }).parentElement!;
    expect(footer).toHaveAttribute('data-sheet-footer');
    expect(footer.previousElementSibling).toHaveTextContent('Body');
    expect(Object.values(useClearanceStore.getState().bars)).toEqual([57]);
    expect(renderHook(() => useFloatingLift()).result.current).toEqual({ bottom: 73 });
  });

  it('marks a footer deeper in its body, and leaves no mark once closed', () => {
    layOut(60);
    const { unmount } = render(
      <Sheet id="pack" icon="package" title="Export pack" onClose={vi.fn()}>
        <SheetFooter className="border-t px-5">
          <button type="button">Export</button>
        </SheetFooter>
      </Sheet>
    );
    expect(Object.values(useClearanceStore.getState().bars)).toEqual([60]);
    unmount();
    expect(useClearanceStore.getState().bars).toEqual({});
  });

  it('sits above the higher bar when a view bar shows under the sheet too', () => {
    layOut(57);
    render(
      <Sheet id="apply" icon="cube" title="Apply" onClose={vi.fn()} footer="Apply">
        <p>Body</p>
      </Sheet>
    );
    useClearanceStore.getState().set('rig-doctor-bar', 114);
    expect(renderHook(() => useFloatingLift()).result.current).toEqual({ bottom: 130 });
  });
});
