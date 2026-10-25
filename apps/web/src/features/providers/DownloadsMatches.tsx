/**
 * Check Downloads: the new GLB and zip files in the Downloads folder,
 * each with the waiting send it most likely is. Nothing is taken until
 * you press Use on a row, and then the file is copied, never moved.
 */
import { useRef } from 'react';
import { Button, Icon, IconButton, InlineBanner, Select } from '@maquette/ui';
import type { DownloadMatch, ProviderJob } from '@/ipc/client';
import { formatAgo, formatBytes, homePath } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useTopEscape } from '@/lib/useTopEscape';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';

const NONE = '';

export function DownloadsMatches() {
  const downloads = useProvidersStore(s => s.downloads);
  const close = useProvidersStore(s => s.closeDownloads);
  if (!downloads) return null;
  return <Dialog onClose={close} />;
}

function Dialog({ onClose }: { onClose: () => void }) {
  const downloads = useProvidersStore(s => s.downloads);
  const retry = useProvidersStore(s => s.checkDownloads);
  const home = useAppStore(s => s.boot?.home);
  const ref = useRef<HTMLDivElement>(null);
  const now = useNow(30_000);
  useTopEscape(ref, onClose);
  if (!downloads) return null;
  const scan = downloads.scan;
  const matched = scan?.files.filter(f => downloads.choices[f.path]).length ?? 0;
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
            <p className="truncate text-meta text-fg-mute">
              {scan
                ? [
                    homePath(scan.folder, home),
                    scan.sinceMs
                      ? `since the first handoff, ${formatAgo(scan.sinceMs, now)}`
                      : null,
                    `${scan.files.length} ${scan.files.length === 1 ? 'file' : 'files'}, ${matched} matched`,
                  ]
                    .filter(Boolean)
                    .join(' · ')
                : downloads.error
                  ? 'Could not read the folder'
                  : 'Looking in Downloads'}
            </p>
          </div>
          <IconButton data-close icon="x" label="Close" onClick={onClose} />
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
              {scan.files.map(file => (
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
              ))}
            </ul>
          )}
        </div>
        <footer className="flex items-center gap-2 border-t border-line px-5 py-3">
          <p className="min-w-0 flex-1 text-meta text-fg-mute">
            I copy each file you use into the inbox. The original stays in Downloads.
          </p>
          <Button variant="secondary" onClick={() => void retry()}>
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

function MatchRow({
  file,
  waiting,
  choice,
  used,
  taken,
  now,
}: {
  file: DownloadMatch;
  waiting: ProviderJob[];
  choice: string | null;
  used: boolean;
  /** Sends another row is set to. */
  taken: string[];
  now: number;
}) {
  const choose = useProvidersStore(s => s.chooseMatch);
  const use = useProvidersStore(s => s.useDownload);
  const busy = useProvidersStore(s => (choice ? s.busy.includes(choice) : false));
  const target = waiting.find(j => j.id === choice);
  const options = [
    { value: NONE, label: 'Not one of these' },
    ...waiting
      .filter(j => j.id === choice || !taken.includes(j.id))
      .map(j => ({ value: j.id, label: j.assetName })),
  ];
  const meta = [
    file.inner ? `holds ${file.inner}` : null,
    file.bytes !== null ? formatBytes(file.bytes) : null,
    file.modifiedMs !== null ? formatAgo(file.modifiedMs, now) : null,
  ]
    .filter(Boolean)
    .join(' · ');
  return (
    <li className="flex items-center gap-3 border-t border-line px-3 py-2.5 first:border-t-0">
      <span className="grid size-9 flex-none place-items-center rounded-control bg-studio text-fg-mute">
        <Icon name={file.inner ? 'package' : 'cube'} size={17} />
      </span>
      <div className="min-w-0 flex-1">
        <div className="truncate font-mono text-[12.5px]" title={file.path}>
          {file.name}
        </div>
        <div className="truncate text-meta text-fg-mute">{meta}</div>
        {file.reason && choice === file.job ? (
          <div className="truncate text-meta text-sage">{file.reason}</div>
        ) : null}
      </div>
      {used ? (
        <span className="flex items-center gap-1.5 text-meta text-sage">
          <Icon name="check" size={14} /> Copied for {target?.assetName ?? file.assetName}
        </span>
      ) : (
        <>
          <Select
            aria-label={`Send for ${file.name}`}
            size="sm"
            options={options}
            value={choice ?? NONE}
            onChange={value => choose(file.path, value || null)}
            fieldClassName="w-[170px]"
          />
          <Button
            size="sm"
            variant={choice ? 'primary' : 'secondary'}
            disabled={!choice || busy}
            loading={busy}
            onClick={() => void use(file.path)}
          >
            Use
          </Button>
        </>
      )}
    </li>
  );
}
