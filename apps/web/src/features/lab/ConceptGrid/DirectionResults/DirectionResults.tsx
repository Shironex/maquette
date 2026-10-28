import { Button, Icon, IconButton, cn } from '@maquette/ui';
import type { AssetKind, Direction, LabView } from '@/ipc/client';
import { Menu } from '@/features/common/Menu';
import { directionSummary } from '@/state/lab';
import { ImageTile } from '../ImageTile';
import { JobTile } from '../JobTile';
import { useDirectionResults } from './DirectionResults.hooks';

const KINDS: readonly { id: AssetKind; label: string }[] = [
  { id: 'scene', label: 'As a scene' },
  { id: 'prop', label: 'As a prop' },
  { id: 'character', label: 'As a character' },
];

export interface DirectionResultsProps {
  view: LabView;
  direction: Direction;
}

/** The selected direction's header with the send actions, and its tiles. */
export default function DirectionResults({ view, direction }: DirectionResultsProps) {
  const {
    busy,
    contactSheet,
    sendPicks,
    objects,
    from,
    kindMenu,
    setKindMenu,
    cells,
    picked,
    known,
    send,
    unsent,
    sendTo3d,
    to3d,
    chip,
  } = useDirectionResults(view, direction);
  const kindItems = KINDS.map(k => ({
    id: k.id,
    label: k.label,
    onSelect: () => void (kindMenu === '3d' ? sendTo3d(k.id) : sendPicks(k.id)),
  }));
  const tileItems = cells.map(cell => (
    <li key={cell.key}>
      {cell.kind === 'image' ? (
        <ImageTile view={view} direction={direction} image={cell.image} />
      ) : (
        <JobTile direction={direction} job={cell.job} />
      )}
    </li>
  ));

  return (
    <section
      aria-label={`${direction.name} concepts`}
      className="flex min-h-0 min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-[22px] pt-[18px] pb-[18px]"
    >
      <div className="flex flex-wrap items-start gap-x-3 gap-y-2.5">
        {/* The title takes the free width and wraps; the actions drop below it only when they must. */}
        <div className="min-w-[min(100%,260px)] flex-1">
          <h2 className="text-screen-title break-words">{direction.name}</h2>
          <p className="mt-0.5 text-fg-mute">
            {directionSummary(direction, view.jobs)}
            {objects ? ` · from ${from?.name ?? direction.source}` : null}
            {chip ? (
              <span
                title={chip.tip}
                className={cn(
                  'ml-2 inline-flex h-5 items-center gap-1 rounded-pill px-2 align-[1px] text-[11.5px] font-medium',
                  chip.on ? 'bg-clay-soft text-clay-text' : 'bg-raised text-fg-mute'
                )}
              >
                <Icon name="palette" size={12} />
                {chip.label}
              </span>
            ) : null}
          </p>
        </div>
        <div className="ml-auto flex flex-none items-start gap-2">
          <IconButton
            icon="squares-four"
            variant="secondary"
            label="Make a contact sheet of every image"
            disabled={direction.images.length === 0 || busy === 'sheet'}
            disabledReason={busy === 'sheet' ? 'Making the sheet' : 'No images yet'}
            onClick={() => void contactSheet()}
          />
          <div className="relative flex gap-2">
            <Button
              icon="cube"
              disabled={picked === 0 || busy === 'send'}
              disabledReason={picked === 0 ? 'Pick at least one concept first' : undefined}
              tooltip={
                objects
                  ? 'Makes one asset per pick, then sends them all in one batch'
                  : unsent
                    ? 'Sends the picks to Refs, then opens Send to 3D'
                    : 'Opens Send to 3D with the picks'
              }
              aria-haspopup={known || objects ? undefined : 'menu'}
              onClick={to3d}
            >
              Send to 3D
            </Button>
            <Button
              variant="primary"
              iconRight="arrow-right"
              loading={busy === 'send'}
              disabled={picked === 0 || busy === 'send'}
              disabledReason={picked === 0 ? 'Pick at least one concept first' : undefined}
              tooltip={
                objects
                  ? 'Copies each pick into refs/ as its own asset, named after the object'
                  : known
                    ? `Copies the picks into refs/ and moves ${known.name} to Refs`
                    : 'Copies the picks into refs/ and adds the asset at Refs'
              }
              aria-haspopup={known || objects ? undefined : 'menu'}
              onClick={send}
            >
              {objects && picked > 0
                ? `Send ${picked} ${picked === 1 ? 'pick' : 'picks'} to Refs`
                : 'Send picks to Refs'}
            </Button>
            {kindMenu ? (
              <Menu
                label="Add the asset"
                onClose={() => setKindMenu(null)}
                className="absolute top-10 right-0 z-30 w-[240px]"
                header={
                  <p className="px-2.5 pt-1.5 pb-1 text-meta text-fg-mute">
                    {kindMenu === '3d'
                      ? `Adds ${direction.name} at Refs, then opens Send to 3D`
                      : `Adds ${direction.name} to the Board at Refs`}
                  </p>
                }
                items={kindItems}
              />
            ) : null}
          </div>
        </div>
      </div>
      <ul className="grid list-none grid-cols-2 gap-3.5">{tileItems}</ul>
    </section>
  );
}
