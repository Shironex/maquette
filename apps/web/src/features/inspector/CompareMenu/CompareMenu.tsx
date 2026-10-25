import { Button, Icon } from '@maquette/ui';
import type { InspectView } from '@/ipc/client';
import { versusLabel, type SidePick } from '@/state/inspector';
import { Menu } from '@/features/common/Menu';
import { useCompareMenu } from './CompareMenu.hooks';

export interface CompareMenuProps {
  view: InspectView;
  pick: SidePick;
}

/** The versions compared, as a button that opens the other comparisons and versions. */
export default function CompareMenu({ view, pick }: CompareMenuProps) {
  const { open, toggle, close, items } = useCompareMenu(view, pick);
  if (items.length === 0) return null;
  return (
    <div className="relative">
      <Button size="sm" onClick={toggle} aria-expanded={open}>
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
        <Menu
          label="Compare"
          items={items}
          onClose={close}
          className="absolute top-full left-0 z-20 mt-1"
        />
      ) : null}
    </div>
  );
}
