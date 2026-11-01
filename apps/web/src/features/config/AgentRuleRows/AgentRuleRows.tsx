import { Toggle, cn } from '@maquette/ui';
import { patchSection } from '@/state/settings-form';
import type { ConfigFieldsProps } from '../ConfigFieldsProps';
import { RULES } from './rules';

export type AgentRuleRowsProps = ConfigFieldsProps;

export default function AgentRuleRows({ config, onChange, density = 'roomy' }: AgentRuleRowsProps) {
  const compact = density === 'compact';
  const rules = config.agents.rules;
  // The first run keeps to the safety rules; the tool list is a later choice.
  const shown = compact ? RULES : RULES.filter(rule => !('settingsOnly' in rule));
  const rows = shown.map(rule => (
    <div
      key={rule.key}
      className={cn(
        'flex items-center gap-4 border-t border-line py-1.5 first:border-t-0',
        compact ? 'min-h-[38px]' : 'min-h-14'
      )}
    >
      <div className="min-w-0 flex-1">
        <b className="block leading-[18px] font-medium text-fg">{rule.title}</b>
        {compact && !('settingsOnly' in rule) ? null : (
          <span className="mt-0.5 block text-meta text-fg-mute">{rule.detail}</span>
        )}
      </div>
      <Toggle
        aria-label={rule.title}
        checked={rules[rule.key] ?? true}
        onChange={on =>
          onChange(patchSection(config, 'agents', { rules: { ...rules, [rule.key]: on } }))
        }
      />
    </div>
  ));
  return <div className="flex flex-col">{rows}</div>;
}
