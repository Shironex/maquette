/**
 * The config controls shared by onboarding and Settings. Each block renders
 * fields only; the screen decides the panel around them.
 */
import { Button, NumberField, SegmentedControl, Select, Toggle, cn } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';
import { formatCount, formatK } from '@/lib/format';
import { mcpSnippet, parsePort, patchSection, type ConfigErrors } from '@/state/settings-form';
import { useAppStore } from '@/state/app-store';
import { CODEC, MESHOPT, RIG, TEXTURE, TOPOLOGY, modelOptions } from './options';

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

export function TripoFields({
  config,
  onChange,
  errors = {},
  density = 'roomy',
}: ConfigFieldsProps) {
  const { tripo } = config;
  const set = (patch: Partial<WorkspaceConfig['tripo']>) =>
    onChange(patchSection(config, 'tripo', patch));
  const setCharacter = (patch: Partial<WorkspaceConfig['tripo']['character']>) =>
    set({ character: { ...tripo.character, ...patch } });
  const compact = density === 'compact';

  return (
    <>
      <div className={cn('grid grid-cols-3', compact ? 'gap-2.5' : 'gap-3')}>
        <Select
          label="Model"
          options={modelOptions(tripo.model)}
          value={tripo.model}
          onChange={model => set({ model })}
        />
        <Select
          label="Topology"
          options={TOPOLOGY}
          value={tripo.topology}
          onChange={topology => set({ topology })}
        />
        <Select
          label="Texture"
          options={TEXTURE}
          value={tripo.texture}
          onChange={texture => set({ texture })}
        />
      </div>
      <FieldError text={errors.model} />
      <div className={cn('flex flex-wrap', compact ? 'gap-[22px]' : 'gap-6')}>
        <Toggle label="HD" checked={tripo.hd} onChange={hd => set({ hd })} />
        <Toggle
          label="Remove lighting"
          checked={tripo.removeLighting}
          onChange={removeLighting => set({ removeLighting })}
        />
        <Toggle label="PBR" checked={tripo.pbr} onChange={pbr => set({ pbr })} />
      </div>
      <div
        className={cn(
          'rounded-[8px] border border-line bg-raised',
          compact
            ? 'flex flex-col gap-[9px] px-3 py-2.5'
            : 'flex items-center gap-[18px] px-3.5 py-3'
        )}
      >
        <div className={cn(compact ? 'flex items-center gap-2' : 'w-[170px] flex-none')}>
          <div className="font-semibold">Character preset</div>
          <div className="text-meta text-fg-mute">
            {compact ? 'used when a ref is tagged character' : 'when a ref is tagged character'}
          </div>
        </div>
        <div
          className={cn(
            compact
              ? 'grid grid-cols-2 items-center gap-x-4 gap-y-2.5'
              : 'flex flex-1 flex-wrap items-center gap-[18px]'
          )}
        >
          <NumberField
            label="Quads"
            inline
            size="sm"
            value={tripo.character.quads}
            min={1}
            format={formatK}
            parse={parseCount}
            onChange={quads => setCharacter({ quads })}
            className={compact ? 'w-[76px]' : 'h-[30px] w-[72px]'}
          />
          <Toggle
            label="Multi-view"
            checked={tripo.character.multiView}
            onChange={multiView => setCharacter({ multiView })}
          />
          <Toggle
            label="Auto rig"
            checked={tripo.character.autoRig}
            onChange={autoRig => setCharacter({ autoRig })}
          />
          <Select
            label={compact ? 'Rig preset' : 'Rig'}
            inline
            size="sm"
            options={RIG}
            value={tripo.character.rigPreset}
            onChange={rigPreset => setCharacter({ rigPreset })}
            className={compact ? 'w-[112px]' : 'h-[30px] w-[112px]'}
          />
        </div>
      </div>
      <FieldError text={errors.quads} />
    </>
  );
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
    </>
  );
}

async function copyText(text: string): Promise<boolean> {
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
}: ConfigFieldsProps) {
  const notify = useAppStore(s => s.notify);
  const port = config.agents.mcpPort;
  const compact = density === 'compact';
  const copy = async (agent: 'claude' | 'codex') => {
    const ok = await copyText(mcpSnippet(agent, port));
    const name = agent === 'claude' ? 'Claude Code' : 'Codex';
    notify(
      ok
        ? {
            status: 'done',
            title: `${name} config copied`,
            detail:
              agent === 'claude'
                ? 'Paste it into .mcp.json.'
                : 'Paste it into ~/.codex/config.toml.',
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
        <Button
          icon={compact ? undefined : 'copy'}
          className="h-[34px]"
          onClick={() => void copy('claude')}
        >
          {compact ? 'Claude Code config' : 'Copy config for Claude Code'}
        </Button>
        <Button
          icon={compact ? undefined : 'copy'}
          className="h-[34px]"
          onClick={() => void copy('codex')}
        >
          {compact ? 'Codex config' : 'Copy config for Codex'}
        </Button>
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

/** The literal snippet, with keys in clay as the canvas shows it. */
export function McpSnippetPreview({ port }: { port: number }) {
  const url = `http://127.0.0.1:${port}/mcp`;
  return (
    <pre className="m-0 overflow-x-auto rounded-[8px] border border-line bg-app px-3.5 py-3 font-mono text-[12px] leading-[19px] whitespace-pre text-fg-dim">
      {'{\n  '}
      <span className="text-clay-text">{'"mcpServers"'}</span>
      {': {\n    '}
      <span className="text-clay-text">{'"maquette"'}</span>
      {`: { "type": "http", "url": "${url}" }\n  }\n}`}
    </pre>
  );
}
