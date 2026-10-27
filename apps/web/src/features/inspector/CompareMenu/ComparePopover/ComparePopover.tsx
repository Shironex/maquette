import type { CSSProperties } from 'react';
import { TextField, cn } from '@maquette/ui';
import { optionDomId, type CompareGroup } from '../compare-options';
import { useComparePopover } from './ComparePopover.hooks';
import { CompareSection } from './CompareSection';

export interface ComparePopoverProps {
  label: string;
  groups: CompareGroup[];
  onClose: () => void;
  className?: string;
  style?: CSSProperties;
}

/**
 * A searchable list of the comparisons in groups, as a combobox over a
 * listbox. It scrolls inside its own height cap, which the caller sets.
 */
export default function ComparePopover({
  label,
  groups,
  onClose,
  className,
  style,
}: ComparePopoverProps) {
  const { rootRef, listId, query, search, shown, active, onKeyDown, pick } = useComparePopover(
    groups,
    onClose
  );
  const sections = shown.map(group => (
    <CompareSection key={group.id} group={group} listId={listId} active={active} onPick={pick} />
  ));
  return (
    <div
      ref={rootRef}
      style={style}
      className={cn(
        'z-20 flex w-[320px] flex-col rounded-card border border-line-strong bg-panel shadow-window',
        className
      )}
    >
      <div className="flex-none border-b border-line p-1.5">
        <TextField
          role="combobox"
          aria-label={`Search ${label.toLowerCase()}`}
          aria-expanded="true"
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={active ? optionDomId(listId, active) : undefined}
          placeholder="Search models"
          icon="magnifying-glass"
          autoComplete="off"
          value={query}
          onChange={e => search(e.target.value)}
          onKeyDown={onKeyDown}
        />
      </div>
      <div
        id={listId}
        role="listbox"
        aria-label={label}
        className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-1"
      >
        {shown.length === 0 ? (
          <p className="px-2 py-3 text-[13px] text-fg-mute">
            No model matches &ldquo;{query.trim()}&rdquo;.
          </p>
        ) : null}
        {sections}
      </div>
    </div>
  );
}
