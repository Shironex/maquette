import { useState } from 'react';
import { IconButton, cn } from '@maquette/ui';
import type { Direction, LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { Menu } from '@/features/common/Menu';
import { useAppStore } from '@/state/use-app-store';
import { coverImage, inProject } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/** The composer's list of earlier directions: pick one to reuse its brief, or move it. */
export function EarlierDirections({ view }: { view: LabView }) {
  const [menu, setMenu] = useState<string | null>(null);
  return (
    <div className="flex flex-col gap-1.5 border-t border-line pt-3.5">
      <span className="mb-1 text-caption text-fg-mute uppercase">Earlier directions</span>
      <ul className="flex list-none flex-col">
        {view.directions.map(d => (
          <DirectionRow
            key={d.id}
            view={view}
            direction={d}
            menuOpen={menu === d.id}
            onMenu={open => setMenu(open ? d.id : null)}
          />
        ))}
      </ul>
    </div>
  );
}

function DirectionRow({
  view,
  direction: d,
  menuOpen,
  onMenu,
}: {
  view: LabView;
  direction: Direction;
  menuOpen: boolean;
  onMenu: (open: boolean) => void;
}) {
  const selected = useLabStore(s => s.selected);
  const reuse = useLabStore(s => s.reuse);
  const move = useLabStore(s => s.moveDirection);
  const busy = useLabStore(s => s.busy);
  const projects = useAppStore(s => s.workspace?.projects) ?? [];
  const others = projects.filter(p => p.id !== view.project);
  const generating = view.jobs.some(
    j => j.direction === d.id && (j.state === 'queued' || j.state === 'running')
  );
  const cover = coverImage(d);
  const url = cover ? fileUrl(inProject(view.projectDir, cover.file)) : null;
  const current = d.id === selected;
  const [stage, setStage] = useState<'actions' | 'move'>('actions');
  const toggle = () => {
    setStage('actions');
    onMenu(!menuOpen);
  };

  return (
    <li className="group relative">
      <button
        type="button"
        aria-current={current ? 'true' : undefined}
        onClick={() => reuse(d.id)}
        className={cn(
          '-mx-1.5 flex w-[calc(100%+12px)] cursor-pointer items-center gap-2.5 rounded-control py-1 pr-9 pl-1.5 text-left hover:bg-hover',
          (current || menuOpen) && 'bg-hover'
        )}
      >
        {url ? (
          <img src={url} alt="" className="h-7 w-11 flex-none rounded-[5px] object-cover" />
        ) : (
          <span className="h-7 w-11 flex-none rounded-[5px] bg-raised" />
        )}
        <span className="min-w-0 flex-1 truncate">{d.name}</span>
        <span className="text-meta text-fg-mute tabular-nums">
          {d.images.length} {d.images.length === 1 ? 'image' : 'images'}
        </span>
      </button>
      <span
        className={cn(
          'absolute top-1/2 -right-1 -translate-y-1/2 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100',
          menuOpen && 'opacity-100'
        )}
      >
        <IconButton
          icon="dots-three"
          size="sm"
          label={`More for ${d.name}`}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onClick={toggle}
        />
      </span>
      {menuOpen && stage === 'actions' ? (
        <Menu
          key="actions"
          label={`More for ${d.name}`}
          onClose={() => onMenu(false)}
          className="absolute top-[calc(100%+2px)] right-0 z-30 w-[220px]"
          items={[
            {
              id: 'move',
              label: 'Move to project…',
              icon: 'arrow-bend-up-right',
              keepOpen: true,
              onSelect: () => setStage('move'),
            },
          ]}
        />
      ) : null}
      {menuOpen && stage === 'move' ? (
        <Menu
          key="move"
          label={`Move ${d.name} to project`}
          onClose={() => onMenu(false)}
          className="absolute top-[calc(100%+2px)] right-0 z-30 w-[260px]"
          header={
            <p className="px-2.5 pt-1.5 pb-1 text-meta text-fg-mute">
              <span className="mb-0.5 block text-caption tracking-[0.06em] uppercase">
                Move to project
              </span>
              {generating
                ? 'Images of this direction are still generating. Move it once they are done.'
                : others.length === 0
                  ? 'There is no other project yet. Make one from the project switcher.'
                  : 'Its concepts, picks, stars and Board card go along.'}
            </p>
          }
          items={others.map(p => ({
            id: p.id,
            label: p.name,
            disabled: generating || busy !== null,
            onSelect: () => void move(d.id, p.id),
          }))}
        />
      ) : null}
    </li>
  );
}
