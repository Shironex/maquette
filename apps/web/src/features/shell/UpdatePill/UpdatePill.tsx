import { Icon, Tooltip, cn } from '@maquette/ui';
import { useUpdatePill } from './UpdatePill.hooks';
import type { UpdatePillProps } from './UpdatePill.types';

/** A quiet pill by the logo when a newer version is out. It opens Settings > About. */
export default function UpdatePill({ collapsed }: UpdatePillProps) {
  const { shown, version, ready, open } = useUpdatePill();
  if (!shown) return null;
  const label = ready ? `Maquette ${version} is ready, restart to update` : `Update ${version}`;
  return (
    <Tooltip label={ready ? 'Ready to install' : 'A newer version is out'} side="right">
      <button
        type="button"
        onClick={open}
        aria-label={collapsed ? label : undefined}
        className={cn(
          'inline-flex h-[22px] shrink-0 cursor-pointer items-center gap-1 rounded-pill whitespace-nowrap border border-clay-line bg-clay-soft px-2 text-[11px] font-medium text-clay-text transition-colors duration-120 hover:bg-hover',
          collapsed && 'size-[22px] justify-center px-0'
        )}
      >
        <Icon name={ready ? 'arrows-clockwise' : 'arrow-up'} size={12} />
        {collapsed ? null : <span>Update {version}</span>}
      </button>
    </Tooltip>
  );
}
