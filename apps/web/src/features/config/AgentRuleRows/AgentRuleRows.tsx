import { Toggle, cn } from '@maquette/ui';
import { patchSection } from '@/state/settings-form';
import type { ConfigFieldsProps } from '../ConfigFieldsProps';
import { RULES } from './rules';

export type AgentRuleRowsProps = ConfigFieldsProps;

export default function AgentRuleRows({ config, onChange, density = 'roomy' }: AgentRuleRowsProps) {
  const compact = density === 'compact';
  const rules = config.agents.rules;
  const rows = RULES.map(rule => (
    <div
      key={rule.key}
      className={cn(
        'flex items-center gap-4 border-t border-line py-1.5 first:border-t-0',
        compact ? 'min-h-[38px]' : 'min-h-14'
      )}
    >
      <div className="min-w-0 flex-1">
        <b className="block leading-[18px] font-medium text-fg">{rule.title}</b>
        {compact ? null : (
          <span className="mt-0.5 block text-meta text-fg-mute">{rule.detail}</span>
        )}
      </div>
      <Toggle
        aria-label={rule.title}
        checked={rules[rule.key]}
        onChange={on =>
          onChange(patchSection(config, 'agents', { rules: { ...rules, [rule.key]: on } }))
        }
      />
    </div>
  ));
  return <div className="flex flex-col">{rows}</div>;
}
