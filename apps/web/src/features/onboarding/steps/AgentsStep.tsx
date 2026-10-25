import { Icon, Toggle } from '@maquette/ui';
import { PANEL_TITLE, SectionLabel } from '@/features/common/Panel';
import { McpStatusLine } from '@/features/config/McpStatusLine';
import { AgentRuleRows } from '@/features/config/AgentRuleRows';
import { McpControls } from '@/features/config/McpControls';
import { McpSnippetPreview } from '@/features/config/McpSnippetPreview';
import { patchSection, validateConfig } from '@/state/settings-form';
import type { OnboardingApi } from '../useOnboarding';

export function AgentsStep({ api }: { api: OnboardingApi }) {
  const config = api.draft.config;
  const onChange = (next: typeof config) => api.update({ config: next });
  const errors = validateConfig(config);
  return (
    <>
      <section
        aria-label="Built-in MCP server"
        className="mt-7 flex flex-col gap-4 rounded-card border border-line bg-panel p-4"
      >
        <div className="flex items-center gap-3.5">
          <span className="grid size-10 place-items-center rounded-[10px] border border-line bg-raised">
            <Icon name="plugs-connected" size={20} />
          </span>
          <div className="flex-1">
            <h2 className={PANEL_TITLE}>Built-in MCP server</h2>
            <McpStatusLine config={config} className="mt-0.5" />
          </div>
          <Toggle
            label="Enabled"
            checked={config.agents.mcpEnabled}
            onChange={mcpEnabled => onChange(patchSection(config, 'agents', { mcpEnabled }))}
          />
        </div>
        <McpControls config={config} onChange={onChange} errors={errors} showCopy={false} />
        <McpSnippetPreview port={config.agents.mcpPort} />
      </section>
      <SectionLabel className="mt-7">Rules agents follow</SectionLabel>
      <div className="mt-2.5 rounded-card border border-line bg-panel px-4">
        <AgentRuleRows config={config} onChange={onChange} />
      </div>
    </>
  );
}
