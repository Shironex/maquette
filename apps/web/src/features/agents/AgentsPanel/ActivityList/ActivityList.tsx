import { Button } from '@maquette/ui';
import type { McpStatus } from '@/ipc/client';
import { useActivityList } from './ActivityList.hooks';
import { ActivityRow } from './ActivityRow';

export interface ActivityListProps {
  status: McpStatus | null;
}

export default function ActivityList({ status }: ActivityListProps) {
  const { calls, clear } = useActivityList(status);
  const rows = calls.map(call => <ActivityRow key={call.id} call={call} />);
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <h3 className="text-meta font-semibold tracking-[0.02em] text-fg-mute uppercase">
          Activity
        </h3>
        {calls.length ? (
          <Button
            size="sm"
            variant="ghost"
            tooltip="Clears the list of tool calls"
            onClick={() => void clear()}
          >
            Clear
          </Button>
        ) : null}
      </div>
      {calls.length === 0 ? (
        <p className="rounded-control border border-dashed border-line px-3 py-3 text-meta text-fg-mute">
          No calls yet. Each tool call an agent makes shows up here.
        </p>
      ) : (
        <ol
          tabIndex={0}
          aria-label="MCP activity"
          className="flex max-h-[260px] list-none flex-col overflow-y-auto rounded-control border border-line bg-app"
        >
          {rows}
        </ol>
      )}
    </div>
  );
}
