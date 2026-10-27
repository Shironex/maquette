import { Button, Select, cn, type SelectOption } from '@maquette/ui';
import type { LayoutView } from '@/ipc/client';
import {
  MAX_PATHS,
  anchorAt,
  energizeLine,
  pathBytes,
  pathLength,
  setAnchor,
  tubeTris,
  type PathEnd,
} from '@/state/paths';
import { Hairline } from '../Hairline';
import { SectionTitle } from '../SectionTitle';
import { newPath } from '../PathsEditor/PathsEditor.hooks';
import { PathEnergize } from './PathEnergize';
import { PathLook } from './PathLook';
import { usePathsPanel } from './PathsPanel.hooks';

const NONE = '';
const END_LABEL: Record<PathEnd, string> = { start: 'Start follows', end: 'End follows' };

export interface PathsPanelProps {
  view: LayoutView;
}

/** The Paths editor's side panel: every path, then the selected one's look, anchors and energize. */
export default function PathsPanel({ view }: PathsPanelProps) {
  const { draft, path, pick, change, remove } = usePathsPanel();
  const stops = view.layout.stops;
  const track = path ? draft.tracks[path.id] : undefined;
  const tris = path ? (tubeTris(path) / 1000).toFixed(1) : '';
  const models: SelectOption<string>[] = [
    { value: NONE, label: 'Nothing' },
    ...view.models.map(m => ({ value: m.asset, label: m.name })),
  ];
  const rows = draft.paths.map(p => {
    const on = path?.id === p.id;
    const t = draft.tracks[p.id];
    return (
      <li key={p.id}>
        <button
          type="button"
          aria-pressed={on}
          onClick={() => pick(p.id)}
          className={cn(
            'flex w-full cursor-pointer items-start gap-2 rounded-control px-2 py-1.5 text-left hover:bg-hover',
            on && 'bg-clay-soft hover:bg-clay-soft'
          )}
        >
          <span
            className="mt-1 size-[9px] flex-none rounded-full"
            style={{ background: p.color, boxShadow: t ? `0 0 6px ${p.glow}` : undefined }}
          />
          <span className="flex min-w-0 flex-1 flex-col">
            <span className={cn('truncate', on && 'text-clay-text')}>{p.name || 'Unnamed'}</span>
            <span className="truncate text-meta text-fg-mute">
              {pathLength(p).toFixed(1)} m · {t ? energizeLine(t, stops) : 'unlit'}
            </span>
          </span>
          <span className="flex-none font-mono text-[11.5px] text-fg-mute">{pathBytes(p, t)}</span>
        </button>
      </li>
    );
  });
  const anchor = (end: PathEnd) => {
    if (!path) return null;
    const value = path[end] ?? NONE;
    return (
      <div className="flex min-h-7 items-center justify-between gap-2">
        <span className="text-fg-dim">{END_LABEL[end]}</span>
        <Select
          aria-label={END_LABEL[end]}
          size="sm"
          options={models}
          value={value}
          fieldClassName="w-[160px]"
          onChange={asset => {
            const model = view.models.find(m => m.asset === asset);
            change((d, id) =>
              setAnchor(
                d,
                id,
                end,
                model?.asset ?? null,
                model ? anchorAt(view.layout, model) : null
              )
            );
          }}
        />
      </div>
    );
  };
  return (
    <aside
      aria-label="Path details"
      className="flex w-[340px] flex-none flex-col gap-4 overflow-y-auto border-l border-line bg-panel px-5 py-[18px]"
    >
      <div className="flex flex-col gap-2">
        <SectionTitle aside={`${draft.paths.length} of ${MAX_PATHS}`}>Paths</SectionTitle>
        {rows.length > 0 ? (
          <ul className="-mx-2 flex flex-col">{rows}</ul>
        ) : (
          <p className="text-meta text-fg-mute">No paths yet.</p>
        )}
        <Button
          size="sm"
          icon="plus"
          className="self-start"
          disabled={draft.paths.length >= MAX_PATHS}
          disabledReason={`A scene keeps ${MAX_PATHS} paths at most`}
          tooltip="Adds a path the site draws as a tube through the scene"
          onClick={newPath}
        >
          New path
        </Button>
      </div>
      {path ? (
        <>
          <Hairline />
          <PathLook path={path} change={change} />
          <Hairline />
          <div className="flex flex-col gap-2.5">
            <SectionTitle>Anchors</SectionTitle>
            {anchor('start')}
            {anchor('end')}
            <p className="text-meta text-fg-mute">
              An anchored end moves with its model when the layout moves it.
            </p>
          </div>
          <Hairline />
          <PathEnergize path={path} track={track} stops={stops} change={change} />
          <Hairline />
          <div className="flex flex-col gap-2">
            <SectionTitle>On the site</SectionTitle>
            <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[12.5px]">
              <dt className="text-fg-mute">Ships as</dt>
              <dd className="text-right">layout.json + tube</dd>
              <dt className="text-fg-mute">Tube</dt>
              <dd className="text-right">{tris}k tris, 1 draw call</dd>
              <dt className="text-fg-mute">Size</dt>
              <dd className="text-right">{pathBytes(path, track)}</dd>
            </dl>
          </div>
          <Button
            size="sm"
            icon="trash"
            variant="danger"
            className="self-start"
            tooltip="Deletes this path and its fill"
            onClick={remove}
          >
            Remove path
          </Button>
        </>
      ) : null}
    </aside>
  );
}
