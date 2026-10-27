import { Button, Icon, cn } from '@maquette/ui';
import type { InspectView } from '@/ipc/client';
import { versusLabel, type SidePick } from '@/state/inspector';
import { ComparePopover } from './ComparePopover';
import { useCompareMenu } from './CompareMenu.hooks';

export interface CompareMenuProps {
  view: InspectView;
  pick: SidePick;
}

/** The versions compared, as a button that opens the other comparisons and versions. */
export default function CompareMenu({ view, pick }: CompareMenuProps) {
  const { open, toggle, close, groups, anchorRef, placement } = useCompareMenu(view, pick);
  if (groups.length === 0) return null;
  return (
    <div ref={anchorRef} className="relative">
      <Button
        size="sm"
        tooltip="Choose which versions to compare"
        onClick={toggle}
        aria-expanded={open}
        aria-haspopup="listbox"
      >
        {pick.left ? (
          <>
            <span className="font-mono">{pick.left.label}</span>
            <span className="text-fg-mute">vs</span>
            <span className="font-mono">{pick.right.label}</span>
          </>
        ) : (
          <span className="font-mono">{versusLabel(pick)}</span>
        )}
        <Icon name="caret-down" size={12} className="text-fg-mute" />
      </Button>
      {open ? (
        <ComparePopover
          label="Compare"
          groups={groups}
          onClose={close}
          style={{ maxHeight: placement.maxHeight }}
          className={cn('absolute left-0', placement.up ? 'bottom-full mb-1' : 'top-full mt-1')}
        />
      ) : null}
    </div>
  );
}
