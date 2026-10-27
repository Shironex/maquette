import { IconButton, Tooltip } from '@maquette/ui';
import { copyExportName } from '../../copyExportName';

export interface ExportNameProps {
  /** The name the site's export is saved under, without an extension. */
  name: string;
}

/**
 * The name a By hand send waits for, on its card, with a copy button, so a
 * round on the site needs no menu. A long name ends in an ellipsis and
 * shows whole in a tooltip.
 */
export default function ExportName({ name }: ExportNameProps) {
  return (
    <div className="-mr-1 flex min-w-0 items-center gap-0.5">
      <Tooltip label={name} side="bottom">
        <code className="min-w-0 flex-1 truncate font-mono text-[11px] leading-[15px] text-fg-dim">
          {name}
        </code>
      </Tooltip>
      <IconButton
        icon="copy"
        size="sm"
        label="Copy export name"
        hint={name}
        data-card-action
        className="size-[22px] w-[22px] flex-none"
        onClick={() => void copyExportName(name)}
      />
    </div>
  );
}
