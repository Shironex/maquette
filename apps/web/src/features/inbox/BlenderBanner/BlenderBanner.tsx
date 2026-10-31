import { Button, Icon } from '@maquette/ui';
import { useBlenderBanner } from './BlenderBanner.hooks';

/** Says picture match is off while Blender is missing, and what waits for it. */
export default function BlenderBanner() {
  const { missing, held, openSettings } = useBlenderBanner();
  if (!missing) return null;
  return (
    <div
      role="status"
      className="flex flex-none items-center gap-3 border-b border-amber/40 bg-amber-soft px-5 py-2"
    >
      <Icon name="warning" size={16} className="flex-none text-amber" />
      <p className="m-0 min-w-0 flex-1 text-fg-dim">
        <span className="font-semibold text-fg">Picture match is off.</span> Blender was not found
        at its saved path, so new downloads match by name only.
        {held > 0
          ? ` ${held} waiting ${held === 1 ? 'download is' : 'downloads are'} on hold until it is back.`
          : ''}
      </p>
      <Button size="sm" variant="secondary" onClick={openSettings}>
        Open Settings
      </Button>
    </div>
  );
}
