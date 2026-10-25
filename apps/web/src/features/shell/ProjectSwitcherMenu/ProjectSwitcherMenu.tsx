import { ProjectMenu, cn } from '@maquette/ui';
import { useProjectSwitcherMenu } from './ProjectSwitcherMenu.hooks';
import type { ProjectSwitcherMenuProps } from './ProjectSwitcherMenu.types';

/** The switcher's popover, under the switcher or beside it on a collapsed rail. */
export default function ProjectSwitcherMenu({
  anchorRef,
  collapsed,
  shortcut,
}: ProjectSwitcherMenuProps) {
  const menu = useProjectSwitcherMenu(anchorRef);
  if (!menu) return null;
  const { projects, note, openDialog, select, dismiss } = menu;
  return (
    <ProjectMenu
      anchorRef={anchorRef}
      shortcut={shortcut}
      className={cn(
        'absolute z-50',
        collapsed ? 'top-0 left-[calc(100%+14px)]' : 'top-[calc(100%+6px)] left-0'
      )}
      projects={projects}
      note={note}
      actions={[
        { id: 'new', label: 'New project', icon: 'plus', onSelect: () => openDialog('template') },
        {
          id: 'import',
          label: 'Import folder',
          icon: 'folder-open',
          onSelect: () => openDialog('import'),
        },
      ]}
      onSelect={select}
      onClose={dismiss}
    />
  );
}
