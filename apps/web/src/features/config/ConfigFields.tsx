/**
 * The config controls shared by onboarding and Settings. Each block renders
 * fields only; the screen decides the panel around them.
 */
import { Button, NumberField, SegmentedControl, Select, Toggle, cn } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';
import { formatCount } from '@/lib/format';
import {
  mcpSnippet,
  mcpUrl,
  parsePort,
  patchSection,
  type ConfigErrors,
  type McpAgent,
} from '@/state/settings-form';
import { useAppStore } from '@/state/use-app-store';
import { CODEC, MESHOPT, TEXTURE } from './options';

export interface ConfigFieldsProps {
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
  errors?: ConfigErrors;
  /** `roomy` is onboarding, `compact` is the Settings grid. */
  density?: 'roomy' | 'compact';
}

function FieldError({ text }: { text?: string }) {
  return text ? <p className="text-meta text-danger">{text}</p> : null;
}

function errorNote(text?: string) {
  return text ? { tone: 'missing' as const, text } : undefined;
}

/** Reads "50k", "50,000" or "50000". */
export function parseCount(text: string): number | null {
  const t = text
    .trim()
    .toLowerCase()
    .replace(/[,\s_]/g, '');
  const match = t.match(/^(\d+(?:\.\d+)?)(k)?$/);
  if (!match) return null;
  const n = Number(match[1]) * (match[2] ? 1000 : 1);
  return Number.isFinite(n) ? Math.round(n) : null;
}

export function BudgetFields({ config, onChange, errors = {} }: ConfigFieldsProps) {
  const set = (patch: Partial<WorkspaceConfig['budgets']>) =>
    onChange(patchSection(config, 'budgets', patch));
  return (
    <>
      <NumberField
        label="Scene target"
        unit="MB"
        value={config.budgets.sceneMb ?? 0}
        min={0.1}
        max={1000}
        format={n => n.toFixed(1)}
        onChange={sceneMb => set({ sceneMb })}
        note={errorNote(errors.sceneMb)}
      />
      <NumberField
        label="First view target"
        unit="MB"
        value={config.budgets.firstViewMb ?? 0}
        min={0.1}
        max={1000}
        format={n => n.toFixed(1)}
        onChange={firstViewMb => set({ firstViewMb })}
        note={errorNote(errors.firstViewMb)}
        hint={errors.firstViewMb ? undefined : 'What the first camera stop may load.'}
      />
      <NumberField
        label="Tris per model"
        unit="tris"
        value={config.budgets.trisPerModel}
        min={1}
        format={formatCount}
        parse={parseCount}
        onChange={trisPerModel => set({ trisPerModel })}
        note={errorNote(errors.trisPerModel)}
      />
    </>
  );
}

export function CompressionFields({ config, onChange }: ConfigFieldsProps) {
  const set = (patch: Partial<WorkspaceConfig['compression']>) =>
    onChange(patchSection(config, 'compression', patch));
  return (
    <>
      <div className="flex flex-col gap-1.5">
        <span aria-hidden="true" className="text-meta font-medium text-fg-dim">
          Texture codec
        </span>
        <SegmentedControl
          label="Texture codec"
          tone="clay"
          block
          options={CODEC}
          value={config.compression.textureCodec}
          onChange={textureCodec => set({ textureCodec })}
        />
      </div>
      <Select
        label="Meshopt"
        options={MESHOPT}
        value={config.compression.meshopt}
        onChange={meshopt => set({ meshopt })}
      />
      <Select
        label="Largest texture"
        options={TEXTURE}
        value={config.compression.textureSize}
        onChange={textureSize => set({ textureSize })}
      />
    </>
  );
}

const NO_TOKEN = 'The token is made the first time the server starts';

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export function McpControls({
  config,
  onChange,
  errors = {},
  density = 'roomy',
  token,
  showCopy = true,
}: ConfigFieldsProps & {
  /** The server's bearer token; until it exists the copy buttons wait. */
  token: string | null;
  /** Onboarding has no token yet, so it shows a preview instead. */
  showCopy?: boolean;
}) {
  const notify = useAppStore(s => s.notify);
  const port = config.agents.mcpPort;
  const compact = density === 'compact';
  const copy = async (agent: McpAgent) => {
    if (!token) return;
    const ok = await copyText(mcpSnippet(agent, port, token));
    const name = agent === 'claude' ? 'Claude Code' : 'Codex';
    notify(
      ok
        ? {
            status: 'done',
            title: `${name} config copied`,
            detail:
              agent === 'claude'
                ? 'Run it in a terminal. It holds the token, so keep it private.'
                : 'Paste it into ~/.codex/config.toml. It holds the token, so keep it private.',
          }
        : {
            status: 'failed',
            title: 'Could not copy',
            detail: 'The clipboard is not available here.',
          }
    );
  };
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
              title={token ? 'A claude mcp add command with the token' : NO_TOKEN}
              onClick={() => void copy('claude')}
            >
              {compact ? 'Claude Code config' : 'Copy config for Claude Code'}
            </Button>
            <Button
              icon={compact ? undefined : 'copy'}
              className="h-[34px]"
              disabled={!token}
              title={token ? 'A ~/.codex/config.toml table with the token' : NO_TOKEN}
              onClick={() => void copy('codex')}
            >
              {compact ? 'Codex config' : 'Copy config for Codex'}
            </Button>
          </>
        ) : null}
      </div>
      <FieldError text={errors.mcpPort} />
    </div>
  );
}

const RULES = [
  {
    key: 'blenderHeadlessOnly',
    title: 'Blender headless only',
    detail: 'Agents run Blender in the background and never open its window',
  },
  {
    key: 'neverCaptureScreen',
    title: 'Never capture the screen',
    detail: 'Visual checks render to files instead of grabbing a window',
  },
  {
    key: 'askBeforeShipping',
    title: 'Ask before shipping',
    detail: 'Ship waits for you to confirm before it writes to the site repo',
  },
] as const;

export function AgentRuleRows({ config, onChange, density = 'roomy' }: ConfigFieldsProps) {
  const compact = density === 'compact';
  const rules = config.agents.rules;
  return (
    <div className="flex flex-col">
      {RULES.map(rule => (
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
      ))}
    </div>
  );
}

/** The Claude Code command, with the token left out, as a preview. */
export function McpSnippetPreview({ port }: { port: number }) {
  return (
    <pre className="m-0 overflow-x-auto rounded-[8px] border border-line bg-app px-3.5 py-3 font-mono text-[12px] leading-[19px] whitespace-pre text-fg-dim">
      <span className="text-clay-text">claude mcp add</span>
      {` --transport http maquette ${mcpUrl(port)} \\\n  --header "Authorization: Bearer `}
      <span className="text-fg-mute">{'<token from Settings > Agents>'}</span>
      {'"'}
    </pre>
  );
}
