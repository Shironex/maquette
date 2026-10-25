import { Menu } from '@/features/common/Menu';
import { useCardMenu } from './CardMenu.hooks';
import type { CardMenuProps } from './CardMenu.types';

/** A card's menu: its actions, or the projects to copy it to after Copy to project. */
export default function CardMenu(props: CardMenuProps) {
  const { asset, board } = props;
  const { copyItems, items, closeAll } = useCardMenu(props);

  if (board.actions.copyFor === asset.id) {
    return (
      <Menu
        key="copy"
        label={`Copy ${asset.name} to project`}
        onClose={closeAll}
        className="w-[180px]"
        header={
          <p className="px-2.5 pt-1.5 pb-1 text-meta text-fg-mute">
            <span className="mb-0.5 block text-caption tracking-[0.06em] uppercase">
              Copy to project
            </span>
            {board.otherProjects.length === 0
              ? 'There is no other project yet.'
              : 'Refs, versions and sends go along. It stays here too.'}
          </p>
        }
        items={copyItems}
      />
    );
  }
  return (
    <Menu
      key="actions"
      label={`${asset.name} actions`}
      onClose={() => board.actions.setMenuFor(null)}
      items={items}
    />
  );
}
