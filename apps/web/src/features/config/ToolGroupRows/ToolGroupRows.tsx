import { Toggle } from '@maquette/ui';
import type { ConfigFieldsProps } from '../ConfigFieldsProps';
import { isOn, offeredCount, useToolGroups, withGroup } from './ToolGroupRows.hooks';

export type ToolGroupRowsProps = Pick<ConfigFieldsProps, 'config' | 'onChange'>;

const plural = (n: number) => `${n} ${n === 1 ? 'tool' : 'tools'}`;

/**
 * Settings > Agents: which MCP tool groups agents get. Off leaves a group's
 * tools out of the list, so agents pay less context for tools they never use.
 */
export default function ToolGroupRows({ config, onChange }: ToolGroupRowsProps) {
  const { groups, error } = useToolGroups();
  const off = config.agents.rules.toolGroupsOff ?? [];
  const { offered, total } = offeredCount(groups, off);
  const headingId = 'tool-groups-heading';
  const rows = groups.map(group => (
    <div
      key={group.id}
      className="flex min-h-[38px] items-center gap-4 border-t border-line py-1.5 first:border-t-0"
    >
      <div className="min-w-0 flex-1">
        <b className="block leading-[18px] font-medium text-fg">
          {group.title}
          <span className="ml-2 text-meta font-normal text-fg-mute tabular-nums">
            {plural(group.tools)}
            {group.alwaysOn ? ', always on' : ''}
          </span>
        </b>
        <span className="mt-0.5 block text-meta text-fg-mute">{group.about}</span>
      </div>
      <Toggle
        aria-label={`${group.title} tools`}
        checked={isOn(group, off)}
        disabled={group.alwaysOn}
        onChange={on => onChange(withGroup(config, group.id, on))}
      />
    </div>
  ));
  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-1">
      <div className="flex flex-wrap items-baseline justify-between gap-x-3">
        <h3
          id={headingId}
          className="text-meta font-semibold tracking-[0.02em] text-fg-mute uppercase"
        >
          Tool groups
        </h3>
        {groups.length > 0 ? (
          <span className="text-meta text-fg-mute tabular-nums">
            {offered} of {plural(total)} offered
          </span>
        ) : null}
      </div>
      <p className="text-meta text-fg-mute">
        Turn off groups you never use for a shorter tool list. Connected agents are told the list
        changed; one that ignores the notice keeps its list until it reconnects, and its calls to an
        off group are refused.
      </p>
      {error ? (
        <p role="alert" className="text-meta text-danger">
          Could not read the tool groups. {error}
        </p>
      ) : null}
      <div className="flex flex-col">{rows}</div>
    </section>
  );
}
