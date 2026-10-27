import { Button, PathField, cn } from '@maquette/ui';
import { formatBytes } from '@/lib/format';
import { Panel } from '@/features/common/Panel';
import { ACTIVE } from '../sections';
import { SyncHelp } from './SyncHelp';
import { useWorkspaceCache, useWorkspaceRoot } from './WorkspaceSection.hooks';
import type { WorkspaceSectionProps } from './WorkspaceSection.types';

export default function WorkspaceSection({ active, config }: WorkspaceSectionProps) {
  const { root, setRoot, note, switchTo, browse } = useWorkspaceRoot();
  const { cache, clearing, clear } = useWorkspaceCache();

  const { layout } = config;
  const folders = (
    [
      [layout.projects, 'one folder per project'],
      [layout.inbox, 'Tripo models land here'],
      [layout.cache, 'bake and export cache, safe to clear'],
    ] as const
  ).map(([name, text]) => (
    <div key={name} className="flex gap-3">
      <span className="w-[84px] font-mono text-[12px] tracking-[-0.01em]">{name}/</span>
      <span className="text-meta text-fg-mute">{text}</span>
    </div>
  ));
  const cacheSize = cache === null ? 'unknown' : formatBytes(cache);

  return (
    <Panel
      id="settings-workspace"
      title="Workspace"
      className={cn('gap-3 px-4 py-3.5', active && ACTIVE)}
    >
      <PathField
        label="Root folder"
        value={root}
        onChange={setRoot}
        onCommit={path => void switchTo(path)}
        onBrowse={() => void browse()}
        note={note}
      />
      <div className="flex flex-col gap-1.5 rounded-[8px] border border-line bg-app px-3 py-2.5">
        {folders}
      </div>
      <div className="flex items-center gap-3">
        <div className="flex-1">
          Cache
          <span className="ml-1.5 font-mono text-[12px] text-fg-mute">{cacheSize}</span>
        </div>
        <Button
          size="sm"
          loading={clearing}
          disabled={!cache}
          disabledReason="The cache is empty"
          tooltip="Deletes the cached step results; the next run builds them again"
          onClick={() => void clear()}
        >
          Clear cache
        </Button>
      </div>
      <SyncHelp cache={layout.cache} />
    </Panel>
  );
}
