import { Button, NumberField } from '@maquette/ui';
import { parsePort, patchSection } from '@/state/settings-form';
import { useMcpCopy } from './McpControls.hooks';
import type { McpControlsProps } from './McpControls.types';

const NO_TOKEN = 'The token is made the first time the server starts';

export default function McpControls({
  config,
  onChange,
  errors = {},
  density = 'roomy',
  showCopy = true,
}: McpControlsProps) {
  const port = config.agents.mcpPort;
  const compact = density === 'compact';
  const { token, copy } = useMcpCopy(port);
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap items-end gap-2.5">
        <NumberField
          label="Localhost port"
          prefix="127.0.0.1:"
          value={port}
          min={1024}
          max={65535}
          format={String}
          parse={parsePort}
          onChange={mcpPort => onChange(patchSection(config, 'agents', { mcpPort }))}
          fieldClassName={compact ? 'w-[140px]' : 'w-[160px]'}
        />
        {showCopy ? (
          <>
            <Button
              icon={compact ? undefined : 'copy'}
              className="h-[34px]"
              disabled={!token}
              disabledReason={NO_TOKEN}
              tooltip="A claude mcp add command with the token"
              onClick={() => void copy('claude')}
            >
              {compact ? 'Claude Code config' : 'Copy config for Claude Code'}
            </Button>
            <Button
              icon={compact ? undefined : 'copy'}
              className="h-[34px]"
              disabled={!token}
              disabledReason={NO_TOKEN}
              tooltip="A ~/.codex/config.toml table with the token"
              onClick={() => void copy('codex')}
            >
              {compact ? 'Codex config' : 'Copy config for Codex'}
            </Button>
          </>
        ) : null}
      </div>
      {errors.mcpPort ? <p className="text-meta text-danger">{errors.mcpPort}</p> : null}
    </div>
  );
}
