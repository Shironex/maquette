import { useId } from 'react';
import { createPortal } from 'react-dom';
import { cn } from '@maquette/ui';
import { useMenu } from './Menu.hooks';
import type { MenuProps } from './Menu.types';
import { MenuRow } from './MenuRow';
import { sectionRuns } from './menuKeys';
import { MENU_PANEL, MENU_SECTION_LABEL, MENU_SEPARATOR } from './menuStyles';

/**
 * A small popup list, fixed in the window and kept inside it. Arrow keys, Home and End move, Escape and a click
 * outside close it, and so does a pick. Sections are parted by a line, a named
 * one is a labelled group under its heading, and an item with a submenu opens
 * it beside itself.
 */
export default function Menu({ label, items, onClose, header, sections, className }: MenuProps) {
  const { ref, onKeyDown, onClickCapture, place } = useMenu(onClose);
  const baseId = useId();
  const rows = sectionRuns(items).map((run, i) => {
    const first = run[0];
    const heading = first?.section ? sections?.[first.section] : undefined;
    const headingId = `${baseId}-${first?.id}`;
    const row = run.map(item => <MenuRow key={item.id} item={item} onClose={onClose} />);
    return (
      <div key={first?.id} className="contents">
        {i > 0 ? <div role="separator" className={MENU_SEPARATOR} /> : null}
        {heading ? (
          <div role="group" aria-labelledby={headingId}>
            <div id={headingId} className={MENU_SECTION_LABEL}>
              {heading}
            </div>
            {row}
          </div>
        ) : (
          row
        )}
      </div>
    );
  });
  const panel = (
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      onKeyDown={onKeyDown}
      onClickCapture={onClickCapture}
      style={
        place
          ? { left: place.left, top: place.top, width: place.width, maxHeight: place.maxHeight }
          : undefined
      }
      className={cn(
        'max-w-[320px] min-w-[180px]',
        MENU_PANEL,
        place ? 'fixed z-50 overflow-y-auto overscroll-contain' : cn('invisible', className)
      )}
    >
      {header}
      {rows}
    </div>
  );
  // Until it is measured it sits where its caller put it, unseen; then it
  // moves to the body, where no scrolling or clipping parent can cut it.
  return place ? createPortal(panel, document.body) : panel;
}
