import { afterEach, describe, expect, it } from 'vitest';
import { menuItems, moveFocus, startsSection } from './menuKeys';

afterEach(() => {
  document.body.innerHTML = '';
});

function menu() {
  document.body.innerHTML = `
    <div role="menu" id="top">
      <button role="menuitem">One</button>
      <button role="menuitem" disabled>Off</button>
      <button role="menuitem" aria-disabled="true">Why not</button>
      <div><button role="menuitem">Move to</button>
        <div role="menu"><button role="menuitem">Nested</button></div>
      </div>
    </div>`;
  return document.getElementById('top')!;
}

const focused = () => document.activeElement?.textContent;

describe('menu keys', () => {
  it('lists the focusable items of the menu itself', () => {
    expect(menuItems(menu()).map(i => i.textContent)).toEqual(['One', 'Why not', 'Move to']);
  });

  it('moves with the arrows, wrapping, and jumps with Home and End', () => {
    const top = menu();
    expect(moveFocus(top, 'ArrowDown')).toBe(true);
    expect(focused()).toBe('One');
    moveFocus(top, 'ArrowDown');
    expect(focused()).toBe('Why not');
    moveFocus(top, 'End');
    expect(focused()).toBe('Move to');
    moveFocus(top, 'ArrowDown');
    expect(focused()).toBe('One');
    moveFocus(top, 'ArrowUp');
    expect(focused()).toBe('Move to');
    moveFocus(top, 'Home');
    expect(focused()).toBe('One');
    expect(moveFocus(top, 'a')).toBe(false);
  });

  it('starts a section where the section name changes', () => {
    const items = [{ section: 'a' }, { section: 'a' }, { section: 'b' }, {}];
    expect(items.map((_, i) => startsSection(items, i))).toEqual([false, false, true, true]);
  });
});
