import { IconButton } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import type { MenuItem } from '@/features/common/Menu';
import { useMoreMenu } from './RecipeMoreMenu.hooks';

/** A recipe's More button and its menu: in a Recipes row and in the editor. */
export default function RecipeMoreMenu({
  name,
  items,
}: {
  name: string;
  items: readonly MenuItem[];
}) {
  const menu = useMoreMenu();
  return (
    <span className="relative flex-none">
      <IconButton
        icon="dots-three"
        size="sm"
        label={`More for ${name}`}
        aria-haspopup="menu"
        aria-expanded={menu.open}
        onClick={menu.toggle}
      />
      {menu.open ? (
        <Menu
          label={`More for ${name}`}
          onClose={menu.close}
          className="absolute top-[calc(100%+2px)] right-0 z-30 w-[220px]"
          items={items}
        />
      ) : null}
    </span>
  );
}
