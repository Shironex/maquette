import { Checkbox, HelpMark, IconButton, LogConsole, Select, Tabs } from '@maquette/ui';
import type { PipelineView } from '@/ipc/client';
import { formatDuration } from '@/state/pipeline';
import { useRunLog } from './RunLog.hooks';

export interface RunLogProps {
  view: PipelineView;
  step: string;
  onStep: (step: string) => void;
}

/** The docked console: live lines per step, or the step timings. */
export default function RunLog({ view, step, onStep }: RunLogProps) {
  const { tab, setTab, auto, toggleAuto, hideNoise, toggleNoise, lines, copy } = useRunLog(
    view,
    step
  );
  const latest = view.runs[0];
  const live = latest?.state === 'running';
  const options = [
    { value: 'all', label: 'All steps' },
    ...view.def.steps.map(s => ({ value: s.id, label: s.name })),
  ];
  const timings = latest?.steps.filter(s => s.state !== 'skipped') ?? [];
  const longest = Math.max(1, ...timings.map(s => s.durationMs ?? 0));

  const timingRows = timings.map(s => (
    <li key={s.id} className="grid grid-cols-[150px_1fr_64px] items-center gap-3">
      <span className="truncate">{s.name}</span>
      <span className="h-1.5 rounded-full bg-hover">
        <span
          className={
            s.state === 'cached'
              ? 'block h-full rounded-full bg-fg-mute/50'
              : s.state === 'failed'
                ? 'block h-full rounded-full bg-danger'
                : 'block h-full rounded-full bg-sage'
          }
          style={{ width: `${Math.max(2, ((s.durationMs ?? 0) / longest) * 100)}%` }}
        />
      </span>
      <span className="text-right font-mono text-[12px] text-fg-mute tabular-nums">
        {s.state === 'cached' ? 'cached' : (formatDuration(s.durationMs) ?? '')}
      </span>
    </li>
  ));

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-card border border-line bg-[#181715]">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line px-3.5">
        <Tabs
          tabs={[
            { id: 'logs', label: 'Logs' },
            { id: 'timings', label: 'Timings' },
          ]}
          value={tab}
          onChange={setTab}
          className="border-b-0"
        />
        {tab === 'logs' ? (
          <>
            <Select
              aria-label="Show the log of"
              size="sm"
              options={options}
              value={step}
              onChange={onStep}
              className="w-[150px]"
            />
            {live ? (
              <span className="inline-flex items-center gap-1.5 text-[12px] text-amber">
                <i className="size-1.5 animate-pulse rounded-full bg-amber" />
                Live
              </span>
            ) : null}
            <span className="ml-auto inline-flex items-center gap-1">
              <Checkbox
                checked={hideNoise}
                onChange={toggleNoise}
                label={<span className="text-meta text-fg-mute">Hide noise</span>}
              />
              <HelpMark
                label="About hiding noise"
                tip="Hides the tools' routine chatter so errors, warnings and results stand out. Nothing is deleted; untick it to see every line."
                side="bottom"
              />
            </span>
            <Checkbox
              checked={auto}
              onChange={toggleAuto}
              label={<span className="text-meta text-fg-mute">Auto-scroll</span>}
            />
            <IconButton icon="copy" size="sm" variant="ghost" label="Copy log" onClick={copy} />
          </>
        ) : null}
      </div>
      {tab === 'logs' ? (
        <LogConsole
          lines={lines}
          autoScroll={auto}
          empty={
            latest
              ? `No log kept for run ${latest.number}. The next run streams here.`
              : 'No output yet. Run the pipeline to see Blender and the compressor here.'
          }
          className="min-h-0 flex-1 rounded-none border-0 bg-transparent [&>div:first-child]:hidden"
        />
      ) : (
        <ol className="flex min-h-0 flex-1 list-none flex-col gap-2 overflow-auto p-3.5">
          {timings.length === 0 ? <li className="text-fg-mute">No timings yet.</li> : null}
          {timingRows}
        </ol>
      )}
    </div>
  );
}
