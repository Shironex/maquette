import { Button, Icon, IconButton, InlineBanner } from '@maquette/ui';
import { formatAgo, homePath } from '@/lib/format';
import { useDownloadsDialog } from './DownloadsDialog.hooks';
import { MatchRow } from './MatchRow';
import { KEYS } from '@/lib/keys';

export interface DownloadsDialogProps {
  onClose: () => void;
}

export default function DownloadsDialog({ onClose }: DownloadsDialogProps) {
  const { downloads, retry, home, ref, now } = useDownloadsDialog(onClose);
  if (!downloads) return null;
  const scan = downloads.scan;
  const matched = scan?.files.filter(f => downloads.choices[f.path]).length ?? 0;
  const summary = scan
    ? [
        homePath(scan.folder, home),
        scan.sinceMs ? `since the first handoff, ${formatAgo(scan.sinceMs, now)}` : null,
        `${scan.files.length} ${scan.files.length === 1 ? 'file' : 'files'}, ${matched} matched`,
      ]
        .filter(Boolean)
        .join(' · ')
    : downloads.error
      ? 'Could not read the folder'
      : 'Looking in Downloads';
  const rows = scan?.files.map(file => (
    <MatchRow
      key={file.path}
      file={file}
      waiting={scan.waiting}
      now={now}
      choice={downloads.choices[file.path] ?? null}
      used={downloads.used.includes(file.path)}
      taken={Object.entries(downloads.choices)
        .filter(([path, job]) => path !== file.path && job)
        .map(([, job]) => job as string)}
    />
  ));
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(16,15,14,0.55)] p-6"
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-labelledby="downloads-title"
        className="flex max-h-full w-[620px] max-w-full flex-col rounded-card border border-line-strong bg-panel shadow-window"
      >
        <header className="flex items-start gap-3 border-b border-line px-5 pt-[18px] pb-3.5">
          <span className="grid size-9 flex-none place-items-center rounded-[10px] bg-clay-soft text-clay-text">
            <Icon name="download-simple" size={18} />
          </span>
          <div className="min-w-0 flex-1">
            <h2 id="downloads-title" className="truncate text-panel-title">
              New models in Downloads
            </h2>
            <p className="truncate text-meta text-fg-mute">{summary}</p>
          </div>
          <IconButton data-close icon="x" label="Close" shortcut={KEYS.escape} onClick={onClose} />
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          {downloads.error ? (
            <InlineBanner
              tone="danger"
              title="Could not check Downloads"
              detail={downloads.error}
              actions={
                <Button size="sm" onClick={() => void retry()}>
                  Try again
                </Button>
              }
            />
          ) : !scan ? (
            <div className="flex items-center justify-center gap-2 py-8 text-fg-mute">
              <Icon name="circle-notch" spin size={16} /> Looking in Downloads
            </div>
          ) : scan.waiting.length === 0 ? (
            <p className="py-6 text-center text-meta text-fg-mute">
              No By hand send is waiting for a model.
            </p>
          ) : scan.files.length === 0 ? (
            <p className="py-6 text-center text-meta text-fg-mute">
              No new .glb or .zip files since the first handoff. Download the model from the web
              app, then check again.
            </p>
          ) : (
            <ul className="m-0 flex list-none flex-col rounded-card border border-line bg-raised p-0">
              {rows}
            </ul>
          )}
        </div>
        <footer className="flex items-center gap-2 border-t border-line px-5 py-3">
          <p className="min-w-0 flex-1 text-meta text-fg-mute">
            I copy each file you use into the inbox. The original stays in Downloads.
          </p>
          <Button
            variant="secondary"
            tooltip="Looks in your Downloads folder again"
            onClick={() => void retry()}
          >
            Check again
          </Button>
          <Button variant="primary" onClick={onClose}>
            Done
          </Button>
        </footer>
      </div>
    </div>
  );
}
