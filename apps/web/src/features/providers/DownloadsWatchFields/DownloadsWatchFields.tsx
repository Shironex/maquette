/**
 * Settings > 3D providers: the Downloads watch for By hand sends. While a
 * send waits, new .glb, .fbx and .zip files in the folder are matched to it
 * and copied into the inbox. It only ever reads that folder.
 */
import { Button, PathField, Toggle } from '@maquette/ui';
import { homePath } from '@/lib/format';
import { useDownloadsWatchFields } from './DownloadsWatchFields.hooks';

const NOTE_TONE = { ok: 'found', error: 'missing' } as const;

export default function DownloadsWatchFields() {
  const { status, saving, home, line, toggle, pickFolder, resetFolder } = useDownloadsWatchFields();
  const enabled = status?.enabled ?? false;
  const tone = line.tone === 'mute' ? null : NOTE_TONE[line.tone];
  return (
    <div className="flex flex-col gap-2.5">
      <Toggle
        label="Watch for By hand exports"
        description="When a model you made on the Tripo or Meshy site lands here, I copy it into the inbox. The original stays put."
        checked={enabled}
        disabled={!status || saving}
        onChange={toggle}
      />
      <PathField
        label="Folder"
        value={status?.folder ? homePath(status.folder, home) : ''}
        placeholder="Downloads"
        onBrowse={pickFolder}
        browseLabel="Change"
        disabled={!enabled || saving}
        note={tone ? { tone, text: line.text } : undefined}
        hint={tone ? undefined : line.text}
      />
      {status?.custom ? (
        <Button size="sm" variant="ghost" className="self-start" onClick={resetFolder}>
          Use Downloads
        </Button>
      ) : null}
    </div>
  );
}
