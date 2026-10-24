import { useState } from 'react';
import { Button, EmptyState, Icon, IconButton, ProgressBar, StatusChip, cn } from '@maquette/ui';
import type { Asset, AssetKind, ConceptImage, ConceptJob, Direction, LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useNow } from '@/lib/useNow';
import { Menu } from '@/features/common/Menu';
import { useAppStore } from '@/state/app-store';
import {
  aspectCss,
  directionSummary,
  formatElapsed,
  inProject,
  isObjects,
  tileLabel,
  tiles,
} from '@/state/lab';
import { useLabStore } from '@/state/lab-store';
import { useProvidersStore } from '@/state/providers-store';

const NO_ASSETS: readonly Asset[] = [];

const KINDS: readonly { id: AssetKind; label: string }[] = [
  { id: 'scene', label: 'As a scene' },
  { id: 'prop', label: 'As a prop' },
  { id: 'character', label: 'As a character' },
];

/** The middle column: the selected direction's images and the ones still generating. */
export function ConceptGrid({ view }: { view: LabView }) {
  const selected = useLabStore(s => s.selected);
  const direction = view.directions.find(d => d.id === selected) ?? null;
  if (!direction) {
    return (
      <section className="flex min-w-0 flex-1 items-center justify-center p-8">
        <EmptyState
          icon="flask"
          title="No concepts yet"
          body="Name a direction, describe it and press Generate. Codex makes each image in about a minute, a few at a time."
          steps={[
            {
              title: 'Write the brief',
              detail: 'A direction name, the prompt and any references.',
            },
            { title: 'Generate', detail: 'Images land in concepts/ inside the project.' },
            { title: 'Star and pick', detail: 'Send the picks to Refs to start the asset.' },
          ]}
        />
      </section>
    );
  }
  return <DirectionResults view={view} direction={direction} />;
}

function DirectionResults({ view, direction }: { view: LabView; direction: Direction }) {
  const busy = useLabStore(s => s.busy);
  const contactSheet = useLabStore(s => s.contactSheet);
  const sendPicks = useLabStore(s => s.sendPicks);
  const sendObjectPicks = useLabStore(s => s.sendObjectPicks);
  const assets = useAppStore(s => s.workspace?.project?.assets) ?? NO_ASSETS;
  const openPanel = useProvidersStore(s => s.openPanel);
  const openBatch = useProvidersStore(s => s.openBatch);
  const objects = isObjects(direction);
  const from = objects
    ? view.directions.find(d => d.images.some(i => i.file === direction.source))
    : undefined;
  /** Which button opened the kind menu: Refs only, or Refs then 3D. */
  const [kindMenu, setKindMenu] = useState<'refs' | '3d' | null>(null);
  const cells = tiles(direction, view.jobs);
  const picked = direction.images.filter(i => i.picked).length;
  const known = assets.find(
    a => a.name.trim().toLowerCase() === direction.name.trim().toLowerCase()
  );

  const send = () => {
    if (objects) void sendObjectPicks();
    else if (known) void sendPicks(known.kind);
    else setKindMenu(m => (m === 'refs' ? null : 'refs'));
  };
  // Picks already in refs/ go straight to the panel; new ones are sent first.
  const unsent = direction.images.some(i => i.picked && !i.refFile);
  const sendTo3d = async (kind?: AssetKind) => {
    const id = known && !unsent ? known.id : await sendPicks(kind ?? known?.kind ?? 'prop', true);
    if (id) void openPanel(view.project, id);
  };
  const to3d = () => {
    if (objects) {
      // One asset per object, then one batch for all of them.
      void sendObjectPicks(true).then(ids => {
        if (ids?.length) void openBatch(view.project, ids);
      });
    } else if (known) void sendTo3d();
    else setKindMenu(m => (m === '3d' ? null : '3d'));
  };

  return (
    <section
      aria-label={`${direction.name} concepts`}
      className="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-[22px] pt-[18px] pb-[18px]"
    >
      <div className="flex flex-wrap items-start gap-x-3 gap-y-2.5">
        {/* The title takes the free width and wraps; the actions drop below it only when they must. */}
        <div className="min-w-[min(100%,260px)] flex-1">
          <h2 className="text-screen-title break-words">{direction.name}</h2>
          <p className="mt-0.5 text-fg-mute">
            {directionSummary(direction, view.jobs)}
            {objects ? ` · from ${from?.name ?? direction.source}` : null}
          </p>
        </div>
        <div className="ml-auto flex flex-none items-start gap-2">
          <IconButton
            icon="squares-four"
            variant="secondary"
            label="Make a contact sheet of every image"
            disabled={direction.images.length === 0 || busy === 'sheet'}
            onClick={() => void contactSheet()}
          />
          <div className="relative flex gap-2">
            <Button
              icon="cube"
              disabled={picked === 0 || busy === 'send'}
              title={
                picked === 0
                  ? 'Pick at least one concept first'
                  : objects
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
              title={
                picked === 0
                  ? 'Pick at least one concept first'
                  : objects
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
                items={KINDS.map(k => ({
                  id: k.id,
                  label: k.label,
                  onSelect: () => void (kindMenu === '3d' ? sendTo3d(k.id) : sendPicks(k.id)),
                }))}
              />
            ) : null}
          </div>
        </div>
      </div>
      <ul className="grid list-none grid-cols-2 gap-3.5">
        {cells.map(cell => (
          <li key={cell.key}>
            {cell.kind === 'image' ? (
              <ImageTile view={view} direction={direction} image={cell.image} />
            ) : (
              <JobTile direction={direction} job={cell.job} />
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function ImageTile({
  view,
  direction,
  image,
}: {
  view: LabView;
  direction: Direction;
  image: ConceptImage;
}) {
  const mark = useLabStore(s => s.mark);
  const openLightbox = useLabStore(s => s.openLightbox);
  const openObjects = useLabStore(s => s.openObjects);
  const path = inProject(view.projectDir, image.file);
  const url = fileUrl(path);
  const label = tileLabel(image);
  return (
    <figure className="group m-0 flex flex-col gap-[7px]">
      <div
        className={cn(
          'relative overflow-hidden rounded-[10px] bg-raised',
          image.picked
            ? 'shadow-[0_0_0_2px_var(--color-clay)]'
            : 'shadow-[0_0_0_1px_var(--color-line)]'
        )}
        style={{ aspectRatio: aspectCss(direction.aspect) }}
      >
        <button
          type="button"
          aria-label={`Open ${label}`}
          onClick={() => openLightbox(path)}
          className="block size-full cursor-zoom-in"
        >
          {url ? (
            <img
              src={url}
              alt={`${direction.name}, ${label}`}
              className="block size-full object-cover"
            />
          ) : null}
        </button>
        {image.picked ? (
          <span className="pointer-events-none absolute top-2 left-2 inline-flex h-[22px] items-center gap-1 rounded-pill bg-clay px-2 text-[11.5px] font-semibold text-clay-ink">
            <Icon name="check" size={12} />
            Picked
          </span>
        ) : null}
        <Button
          size="sm"
          variant={image.picked ? 'secondary' : 'primary'}
          icon={image.picked ? 'x' : 'check'}
          onClick={() => void mark(image.file, { picked: !image.picked })}
          className="absolute top-2 right-2 opacity-0 transition-opacity duration-120 group-focus-within:opacity-100 group-hover:opacity-100"
        >
          {image.picked ? 'Unpick' : 'Pick'}
        </Button>
      </div>
      <figcaption className="flex items-center justify-between gap-2">
        <span className="min-w-0 truncate font-medium">{label}</span>
        <span className="flex flex-none items-center gap-1.5">
          {image.refFile ? (
            <span className="text-meta whitespace-nowrap text-fg-mute" title={image.refFile}>
              In Refs
            </span>
          ) : null}
          {isObjects(direction) ? null : (
            <IconButton
              icon="cube-focus"
              label={`Make object refs from ${label}`}
              title="Make object refs..."
              variant="ghost"
              size="sm"
              onClick={() =>
                openObjects({
                  project: view.project,
                  source: image.file,
                  sourcePath: path,
                  sourceName: direction.name,
                })
              }
              className="text-fg-mute"
            />
          )}
          <IconButton
            icon={image.starred ? 'star-fill' : 'star'}
            label={image.starred ? `Unstar ${label}` : `Star ${label}`}
            aria-pressed={image.starred}
            variant="ghost"
            size="sm"
            onClick={() => void mark(image.file, { starred: !image.starred })}
            className={image.starred ? 'text-clay-text' : 'text-fg-mute'}
          />
        </span>
      </figcaption>
    </figure>
  );
}

function JobTile({ direction, job }: { direction: Direction; job: ConceptJob }) {
  const now = useNow(1000);
  const running = job.state === 'running';
  return (
    <figure className="m-0 flex flex-col gap-[7px]">
      <div
        className="flex flex-col items-center justify-center gap-2.5 rounded-[10px] border border-line bg-raised"
        style={{ aspectRatio: aspectCss(direction.aspect) }}
      >
        <StatusChip
          status={running ? 'running' : 'queued'}
          label={running ? 'generating' : 'waiting'}
        />
        {running ? <ProgressBar className="w-[140px]" /> : null}
      </div>
      <figcaption className="flex items-center justify-between">
        <span className="min-w-0 truncate text-fg-mute">{tileLabel(job)}</span>
        <span className={cn('text-meta tabular-nums', running ? 'text-amber' : 'text-fg-mute')}>
          {running && job.startedAtMs !== null ? formatElapsed(now - job.startedAtMs) : 'in line'}
        </span>
      </figcaption>
    </figure>
  );
}
