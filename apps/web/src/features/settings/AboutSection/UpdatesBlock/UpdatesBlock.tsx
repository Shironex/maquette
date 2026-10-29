import { Button, Icon, ProgressBar, cn } from '@maquette/ui';
import { formatBytes } from '@/lib/format';
import { useUpdatesBlock } from './UpdatesBlock.hooks';
import type { UpdatesBlockProps } from './UpdatesBlock.types';

export default function UpdatesBlock({ version }: UpdatesBlockProps) {
  const { mode, phase, refused, line, update, percent, check, download, install } =
    useUpdatesBlock(version);
  const on = mode === 'on';
  const busy = phase.kind === 'checking' || phase.kind === 'downloading';

  const action =
    phase.kind === 'available' || (phase.kind === 'failed' && update) ? (
      <Button
        size="sm"
        variant="primary"
        icon="download-simple"
        tooltip="Downloads this version; it installs when you restart"
        onClick={download}
      >
        Download {update?.version}
      </Button>
    ) : phase.kind === 'ready' || phase.kind === 'installing' ? (
      <Button
        size="sm"
        variant="primary"
        icon="arrows-clockwise"
        loading={phase.kind === 'installing'}
        onClick={install}
      >
        Restart to update
      </Button>
    ) : null;

  return (
    <div role="group" className="flex flex-col gap-2.5" aria-labelledby="settings-about-updates">
      <div className="flex items-center gap-2.5">
        <h3 id="settings-about-updates" className="flex-1 font-medium text-fg">
          Updates
        </h3>
        {action}
        <Button
          size="sm"
          icon="arrow-clockwise"
          loading={phase.kind === 'checking'}
          disabled={!on || busy || phase.kind === 'ready' || phase.kind === 'installing'}
          disabledReason={
            !on
              ? 'Updates are off in this build'
              : busy
                ? 'An update step is running'
                : 'An update is ready to install'
          }
          onClick={check}
        >
          Check now
        </Button>
      </div>
      <p
        role="status"
        aria-live="polite"
        className={cn(
          'flex items-start gap-1.5 text-meta',
          phase.kind === 'failed' && on ? 'text-danger' : 'text-fg-dim'
        )}
      >
        <Icon
          name={!on ? 'info' : phase.kind === 'failed' ? 'warning-circle' : 'cloud-arrow-down'}
          size={14}
          className="mt-px"
        />
        <span>{line}</span>
      </p>
      {phase.kind === 'downloading' ? (
        <ProgressBar
          value={percent}
          label="Download"
          valueLabel={
            phase.total
              ? `${formatBytes(phase.downloaded)} of ${formatBytes(phase.total)}`
              : formatBytes(phase.downloaded)
          }
        />
      ) : null}
      {refused ? <p className="text-meta text-amber">{refused}</p> : null}
      {on && update ? (
        <section
          aria-label={`Release notes for ${update.version}`}
          className="flex flex-col gap-1 rounded-[8px] border border-line bg-app px-3 py-2.5"
        >
          <div className="flex items-baseline gap-2 text-meta">
            <span className="font-medium text-fg">What is new in {update.version}</span>
            {update.date ? <span className="text-fg-mute">{update.date}</span> : null}
          </div>
          <div className="max-h-40 overflow-y-auto text-[12px] leading-[18px] whitespace-pre-wrap text-fg-dim">
            {update.notes ?? 'This release came without notes.'}
          </div>
        </section>
      ) : null}
    </div>
  );
}
