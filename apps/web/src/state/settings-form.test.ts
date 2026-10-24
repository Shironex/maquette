import { describe, expect, it } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import {
  describeErrors,
  dropInvalid,
  maskToken,
  mcpSnippet,
  parsePort,
  patchSection,
  saveStatus,
  validateConfig,
} from './settings-form';

describe('settings form', () => {
  it('patches one section and leaves the rest alone', () => {
    const base = defaultConfig();
    const next = patchSection(base, 'budgets', { sceneMb: 4.5 });
    expect(next.budgets).toEqual({ sceneMb: 4.5, trisPerModel: 60000, firstViewMb: 3 });
    expect(next.providers).toBe(base.providers);
    expect(base.budgets.sceneMb).toBe(3);
  });

  it('accepts the defaults and flags each bad field', () => {
    expect(validateConfig(defaultConfig())).toEqual({});
    let bad = patchSection(defaultConfig(), 'agents', { mcpPort: 80 });
    bad = patchSection(bad, 'budgets', { sceneMb: 0, firstViewMb: 0 });
    bad = patchSection(bad, 'layout', { inbox: 'Projects' });
    bad = {
      ...bad,
      providers: {
        ...bad.providers,
        meshy: {
          ...bad.providers.meshy,
          character: {
            ...bad.providers.meshy.character,
            aiModel: 'meshy-t2',
            targetPolycount: 20000,
          },
        },
      },
    };
    const errors = validateConfig(bad, {
      tripoModels: [],
      meshyModels: [{ id: 'meshy-t2', label: 'Meshy T2', note: '' }],
    });
    expect(Object.keys(errors).sort()).toEqual([
      'firstViewMb',
      'layout',
      'mcpPort',
      'meshy-character',
      'sceneMb',
    ]);
    expect(errors['meshy-character']).toBe('Meshy T2 aims for 100 to 15,000 faces.');
  });

  it('reports what the autosave line should say', () => {
    const saved = defaultConfig();
    const flags = { saving: false, failed: false };
    expect(saveStatus(saved, saved, flags)).toBe('saved');
    const edited = patchSection(saved, 'compression', { textureSize: '4K' });
    expect(saveStatus(edited, saved, flags)).toBe('pending');
    expect(saveStatus(edited, saved, { saving: true, failed: false })).toBe('saving');
    expect(saveStatus(edited, saved, { saving: false, failed: true })).toBe('error');
    const invalid = patchSection(saved, 'agents', { mcpPort: 70000 });
    expect(saveStatus(invalid, saved, flags)).toBe('invalid');
  });

  it('builds working agent setups with the token', () => {
    expect(mcpSnippet('claude', 4791, 'abc123')).toBe(
      'claude mcp add --transport http maquette http://127.0.0.1:4791/mcp --header "Authorization: Bearer abc123"\n'
    );
    expect(mcpSnippet('codex', 5000, 'abc123')).toBe(
      [
        '[mcp_servers.maquette]',
        'url = "http://127.0.0.1:5000/mcp"',
        'http_headers = { Authorization = "Bearer abc123" }',
        'tool_timeout_sec = 150',
        '',
      ].join('\n')
    );
    expect(maskToken('0123456789abcdef')).toBe('012345\u2026cdef');
  });

  it('reads a port with or without the localhost prefix', () => {
    expect(parsePort('127.0.0.1:4791')).toBe(4791);
    expect(parsePort(' 5000 ')).toBe(5000);
    expect(parsePort('example.com:80')).toBeNull();
  });

  it('names each bad field, and puts only those back to the saved value', () => {
    const saved = defaultConfig();
    let draft = patchSection(saved, 'agents', { mcpPort: 80 });
    draft = patchSection(draft, 'budgets', { sceneMb: 0, trisPerModel: 70000 });
    const errors = validateConfig(draft);
    expect(describeErrors(errors)).toEqual([
      expect.stringMatching(/^Scene target: /),
      expect.stringMatching(/^Localhost port: /),
    ]);
    const kept = dropInvalid(draft, saved, errors);
    expect(kept.agents.mcpPort).toBe(saved.agents.mcpPort);
    expect(kept.budgets).toEqual({ sceneMb: saved.budgets.sceneMb, trisPerModel: 70000 });
    expect(validateConfig(kept)).toEqual({});
  });
});
