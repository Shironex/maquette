import { Button, Icon, Select } from '@maquette/ui';
import type { DownloadMatch, ProviderJob } from '@/ipc/client';
import { formatAgo, formatBytes } from '@/lib/format';
import { useMatchRow } from './MatchRow.hooks';

const NONE = '';

export interface MatchRowProps {
  file: DownloadMatch;
  waiting: ProviderJob[];
  choice: string | null;
  used: boolean;
  /** Sends another row is set to. */
  taken: string[];
  now: number;
}

export default function MatchRow({ file, waiting, choice, used, taken, now }: MatchRowProps) {
  const { choose, use, busy } = useMatchRow(choice);
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
            disabledReason="Pick the send it belongs to"
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
