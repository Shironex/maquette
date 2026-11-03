import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { MOCK_TOOL_GROUPS } from './mock-tool-groups';

const read = (path: string) => readFileSync(resolve(import.meta.dirname, path), 'utf8');

const toolNames = read('../../../../desktop/src-tauri/tool-names.txt')
  .split(/\r?\n/)
  .map(line => line.trim())
  .filter(line => line && !line.startsWith('#'));

describe('the mock tool groups', () => {
  it('count every tool the server lists, once', () => {
    const total = MOCK_TOOL_GROUPS.reduce((sum, g) => sum + g.tools, 0);
    expect(total).toBe(toolNames.length);
  });

  it('say what the desktop groups say, in their order', () => {
    const source = read('../../../../desktop/src-tauri/src/mcp/tools/groups.rs');
    const ids = [...source.matchAll(/(?:core|group)\(\s*"([a-z-]+)",/g)].map(m => m[1]);
    expect(MOCK_TOOL_GROUPS.map(g => g.id)).toEqual(ids);
    for (const group of MOCK_TOOL_GROUPS) {
      expect(source).toContain(JSON.stringify(group.title));
      expect(source).toContain(JSON.stringify(group.about));
    }
  });

  it('keep start and project always on, and only those', () => {
    expect(MOCK_TOOL_GROUPS.filter(g => g.alwaysOn).map(g => g.id)).toEqual(['start', 'project']);
  });
});
