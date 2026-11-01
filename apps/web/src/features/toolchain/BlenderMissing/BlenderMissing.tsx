import { Button, Icon, IconButton, cn } from '@maquette/ui';
import { useBlenderMissing } from './BlenderMissing.hooks';

export interface BlenderMissingProps {
  className?: string;
}

/** Shows only while Blender is not usable: where to get it and that I will notice it. */
export default function BlenderMissing({ className }: BlenderMissingProps) {
  const { showing, broken, checking, command, getBlender, copyCommand } = useBlenderMissing();
  if (!showing) return null;
  const title = broken ? 'Blender did not run' : 'Blender is missing';
  return (
    <section
      aria-label={title}
      className={cn(
        'flex flex-col gap-2.5 rounded-card border border-amber/40 bg-amber-soft p-3.5',
        className
      )}
    >
      <div className="flex items-start gap-2.5">
        <Icon name="warning" size={16} className="mt-0.5 flex-none text-amber" />
        <div className="min-w-0 flex-1">
          <h2 className="text-[13px] font-semibold">{title}</h2>
          <p className="mt-0.5 text-meta text-fg-dim">
            {broken
              ? 'The copy I found did not answer. Install a fresh one, or locate a working copy below.'
              : 'Every build runs in it. Install it, or locate it below if it is somewhere I did not look.'}{' '}
            Use the current stable release, or a 4.2 LTS or newer.
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          icon="arrow-square-out"
          tooltip="Opens blender.org in your browser"
          onClick={getBlender}
        >
          Get Blender
        </Button>
        {command ? (
          <span className="inline-flex min-w-0 items-center gap-1 rounded-[6px] border border-line bg-app py-0.5 pr-0.5 pl-2">
            <code className="min-w-0 truncate font-mono text-[12px] text-fg-dim">{command}</code>
            <IconButton
              icon="copy"
              size="sm"
              variant="ghost"
              label="Copy the install command"
              onClick={() => void copyCommand()}
            />
          </span>
        ) : null}
        <span className="ml-auto text-meta text-fg-mute" aria-live="polite">
          {checking ? 'Checking...' : 'I check again when you come back to this window.'}
        </span>
      </div>
    </section>
  );
}
