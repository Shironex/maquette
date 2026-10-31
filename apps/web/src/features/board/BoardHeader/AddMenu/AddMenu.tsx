import { Button, IconButton } from '@maquette/ui';
import { Menu, type MenuItem } from '@/features/common/Menu';
import { AddAssetForm } from '../../AddAssetForm';
import { useAddMenu } from './AddMenu.hooks';

const NO_PROJECT = 'Open a project first';

/** Add: the main click adds an asset; the caret lists the other ways to bring one in. */
export default function AddMenu() {
  const { project, game, adding, setAdding, menuOpen, setMenuOpen, added, addBuilt, importModels } =
    useAddMenu();
  const items: MenuItem[] = [
    {
      id: 'asset',
      label: 'Add asset',
      icon: 'plus',
      hint: 'A new card on the Board, named, with its kind and column',
      onSelect: () => setAdding(true),
    },
    ...(game
      ? [
          {
            id: 'import',
            label: 'Import models',
            icon: 'download-simple',
            hint: 'Bring existing game models into this project',
            onSelect: importModels,
          } satisfies MenuItem,
        ]
      : []),
    {
      id: 'built',
      label: 'Built asset',
      icon: 'wrench',
      hint: 'A model a Blender script in the project makes',
      onSelect: addBuilt,
    },
  ];

  return (
    <div className="relative flex items-center">
      <Button
        icon="plus"
        aria-label="Add asset"
        aria-expanded={adding}
        disabled={!project}
        disabledReason={NO_PROJECT}
        tooltip="Add asset: a new card on the Board, named, with its kind and column"
        className="rounded-r-none"
        onClick={() => setAdding(v => !v)}
      >
        Add
      </Button>
      <IconButton
        icon="caret-down"
        label="More ways to add"
        variant="secondary"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        disabled={!project}
        disabledReason={NO_PROJECT}
        className="-ml-px w-7 rounded-l-none"
        // A press on the open menu's own button must not close it and open it again.
        onPointerDown={e => {
          if (menuOpen) e.stopPropagation();
        }}
        onClick={() => setMenuOpen(!menuOpen)}
      />
      {menuOpen ? (
        <Menu
          label="Add to the project"
          onClose={() => setMenuOpen(false)}
          className="absolute top-full right-0 mt-1 w-[200px]"
          items={items}
        />
      ) : null}
      {adding && project ? (
        <AddAssetForm project={project.id} onDone={added} onCancel={() => setAdding(false)} />
      ) : null}
    </div>
  );
}
