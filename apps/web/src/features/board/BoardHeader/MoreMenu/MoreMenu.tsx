import { IconButton } from '@maquette/ui';
import { Menu, type MenuItem } from '@/features/common/Menu';
import { RecentlyRemoved } from '../../RecentlyRemoved';
import { useMoreMenu } from './MoreMenu.hooks';

const SECTIONS = { make: 'Make', project: 'Project' };

/** The ⋯ menu; a small dot says the project's trash holds something. */
export default function MoreMenu() {
  const {
    wrap,
    project,
    open,
    setOpen,
    removed,
    sends,
    failed,
    missing,
    rendering,
    waiting,
    allSends,
    exportPack,
    renderMissing,
  } = useMoreMenu();
  const trash = removed.entries.length;
  const noModel = (project?.assets ?? []).every(a => a.versions.length === 0);
  const items: MenuItem[] = [
    {
      id: 'pack',
      label: 'Export pack',
      icon: 'package',
      section: 'make',
      disabled: !project || noModel,
      hint: !project ? 'Open a project first' : noModel ? 'No model accepted yet' : undefined,
      detail: 'Accepted models, checked in Blender',
      onSelect: exportPack,
    },
    ...(missing > 0
      ? [
          {
            id: 'previews',
            label: 'Render previews',
            icon: 'image',
            section: 'make',
            count: missing,
            disabled: rendering,
            hint: rendering
              ? `Rendering the previews now${waiting > 0 ? `, ${waiting} left` : ''}`
              : 'Renders each model without a preview, one at a time. Run it again to retry the ones that failed',
            detail: 'Headless in Blender',
            onSelect: renderMissing,
          } satisfies MenuItem,
        ]
      : []),
    {
      id: 'sends',
      label: 'All sends',
      icon: 'cube',
      section: 'project',
      count: sends > 0 ? sends : undefined,
      disabled: !project,
      hint: project ? undefined : 'Open a project first',
      detail: failed > 0 ? `${failed} failed` : 'Finished ones too, with Credits',
      onSelect: allSends,
    },
    ...(trash > 0
      ? [
          {
            id: 'removed',
            label: 'Removed',
            icon: 'trash',
            section: 'project',
            count: trash,
            detail: 'In the workspace trash, ready to restore',
            onSelect: () => removed.setOpen(true),
          } satisfies MenuItem,
        ]
      : []),
  ];

  return (
    <div ref={wrap} className="relative">
      <IconButton
        icon="dots-three"
        label="More board actions"
        hint={trash > 0 ? `${trash} recently removed` : undefined}
        variant="secondary"
        aria-haspopup="menu"
        aria-expanded={open || removed.open}
        // A press on the open menu's own button must not close it and open it again.
        onPointerDown={e => {
          if (open) e.stopPropagation();
        }}
        onClick={() => {
          removed.setOpen(false);
          setOpen(!open);
        }}
      />
      {trash > 0 ? (
        <span
          aria-hidden
          data-testid="removed-dot"
          className="pointer-events-none absolute top-1 right-1 size-1.5 rounded-full bg-fg-mute"
        />
      ) : null}
      {open ? (
        <Menu
          label="More board actions"
          onClose={() => setOpen(false)}
          className="absolute top-full right-0 mt-1 w-[272px]"
          sections={SECTIONS}
          items={items}
        />
      ) : null}
      {removed.open ? (
        <RecentlyRemoved
          entries={removed.entries}
          restoring={removed.restoring}
          restoreOne={removed.restoreOne}
          onClose={() => {
            removed.setOpen(false);
            wrap.current?.querySelector('button')?.focus();
          }}
        />
      ) : null}
    </div>
  );
}
