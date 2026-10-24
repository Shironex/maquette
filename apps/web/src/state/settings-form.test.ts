import { describe, expect, it } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { mcpSnippet, parsePort, patchSection, saveStatus, validateConfig } from './settings-form';

describe('settings form', () => {
  it('patches one section and leaves the rest alone', () => {
    const base = defaultConfig();
    const next = patchSection(base, 'budgets', { sceneMb: 4.5 });
    expect(next.budgets).toEqual({ sceneMb: 4.5, trisPerModel: 60000 });
    expect(next.tripo).toBe(base.tripo);
    expect(base.budgets.sceneMb).toBe(3);
  });

  it('accepts the defaults and flags each bad field', () => {
    expect(validateConfig(defaultConfig())).toEqual({});
    let bad = patchSection(defaultConfig(), 'agents', { mcpPort: 80 });
    bad = patchSection(bad, 'budgets', { sceneMb: 0 });
    bad = patchSection(bad, 'layout', { inbox: 'Projects' });
    expect(Object.keys(validateConfig(bad)).sort()).toEqual(['layout', 'mcpPort', 'sceneMb']);
  });

  it('reports what the autosave line should say', () => {
    const saved = defaultConfig();
    const flags = { saving: false, failed: false };
    expect(saveStatus(saved, saved, flags)).toBe('saved');
    const edited = patchSection(saved, 'tripo', { hd: false });
    expect(saveStatus(edited, saved, flags)).toBe('pending');
    expect(saveStatus(edited, saved, { saving: true, failed: false })).toBe('saving');
    expect(saveStatus(edited, saved, { saving: false, failed: true })).toBe('error');
    const invalid = patchSection(saved, 'agents', { mcpPort: 70000 });
    expect(saveStatus(invalid, saved, flags)).toBe('invalid');
  });

  it('builds the agent config snippets for a port', () => {
    expect(JSON.parse(mcpSnippet('claude', 4317))).toEqual({
      mcpServers: { maquette: { type: 'http', url: 'http://127.0.0.1:4317/mcp' } },
    });
    expect(mcpSnippet('codex', 5000)).toBe(
      '[mcp_servers.maquette]\nurl = "http://127.0.0.1:5000/mcp"\n'
    );
  });

  it('reads a port with or without the localhost prefix', () => {
    expect(parsePort('127.0.0.1:4317')).toBe(4317);
    expect(parsePort(' 5000 ')).toBe(5000);
    expect(parsePort('example.com:80')).toBeNull();
  });
});
