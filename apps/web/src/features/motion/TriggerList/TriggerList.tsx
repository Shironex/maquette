import { Button, Icon, IconButton } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import { SectionLabel } from '@/features/common/SectionLabel';
import { useTriggerList } from './TriggerList.hooks';

/** What starts each move on the site: a camera stop, the scroll, a click or a hover. */
export default function TriggerList() {
  const list = useTriggerList();
  if (!list) return null;
  const rows = list.rows.map(row => (
    <div key={row.key} className="flex items-start gap-2.5 border-t border-line py-2.5">
      <Icon name={row.icon} size={14} className="mt-0.5 flex-none text-fg-dim" />
      <div className="min-w-0 flex-1">
        <p className="text-[13px] leading-[18px] font-medium">{row.title}</p>
        <p className="text-meta text-fg-mute">{row.detail}</p>
      </div>
      <IconButton icon="trash" size="sm" label={`Remove ${row.title}`} onClick={row.remove} />
    </div>
  ));
  return (
    <section aria-label="Triggers" className="flex flex-col gap-1">
      <div className="relative flex items-center justify-between gap-2 pb-1">
        <SectionLabel>Triggers · {list.rows.length}</SectionLabel>
        <Button
          size="sm"
          icon="plus"
          aria-expanded={list.menu}
          disabled={!list.target}
          disabledReason="Add a second state first"
          tooltip="Adds something that moves the model between states"
          onClick={() => list.setMenu(!list.menu)}
        >
          Add
        </Button>
        {list.menu ? (
          <Menu
            label="Add a trigger"
            className="absolute top-8 right-0 z-10 w-[240px]"
            onClose={() => list.setMenu(false)}
            items={[
              {
                id: 'stop',
                label: 'Arriving at a camera stop',
                icon: 'video-camera',
                disabled: !list.hasStops,
                hint: list.hasStops ? undefined : 'The layout has no camera stops',
                onSelect: () => list.add('stop'),
              },
              {
                id: 'scroll',
                label: 'Scrolling the page',
                icon: 'mouse-scroll',
                onSelect: () => list.add('scroll'),
              },
              {
                id: 'click',
                label: 'A click on the model',
                icon: 'cursor-click',
                onSelect: () => list.add('click'),
              },
              {
                id: 'hover',
                label: 'A hover on the model',
                icon: 'hand-pointing',
                onSelect: () => list.add('hover'),
              },
            ]}
          />
        ) : null}
      </div>
      {list.rows.length === 0 ? (
        <p className="text-meta text-fg-mute">
          {list.target
            ? `Nothing starts a move yet. Add a trigger to go to ${list.target}.`
            : 'Triggers start the moves between states. Add a second state first.'}
        </p>
      ) : (
        rows
      )}
    </section>
  );
}
