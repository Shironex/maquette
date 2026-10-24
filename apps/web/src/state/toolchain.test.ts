import { describe, expect, it } from 'vitest';
import { toolReports } from '@/ipc/mock/fixtures';
import { displayToolPath, rowStatus, rowSummary, summarizeTools } from './toolchain';

const reports = toolReports('first-run');
const byId = (id: string) => reports.find(r => r.id === id)!;

describe('toolchain rows', () => {
  it('shows an app bundle instead of the binary inside it', () => {
    expect(displayToolPath('/Applications/Blender.app/Contents/MacOS/Blender')).toBe(
      '/Applications/Blender.app'
    );
    expect(displayToolPath('C:\\Program Files\\Blender\\blender.exe')).toBe(
      'C:\\Program Files\\Blender\\blender.exe'
    );
  });

  it('reads found tools as version, location and detail', () => {
    expect(rowSummary(byId('blender'))).toBe('4.3.2 at /Applications/Blender.app, runs headless');
    expect(rowSummary(byId('blender'), 'compact')).toBe('4.3.2 · runs headless');
    expect(rowSummary(byId('gltf-transform'))).toBe('via npx, @gltf-transform/cli');
    expect(rowSummary(byId('toktx'))).toBe('not found on PATH');
  });

  it('marks a found tool with a warning as a warning', () => {
    const codex = { ...byId('codex'), warning: 'Codex CLI is not signed in.' };
    expect(rowStatus(codex)).toBe('warning');
    expect(rowStatus(byId('toktx'))).toBe('missing');
    expect(rowStatus(byId('node'), true)).toBe('checking');
  });

  it('counts found tools and warnings, and blocks only on Blender', () => {
    expect(summarizeTools(reports)).toMatchObject({
      found: 5,
      total: 6,
      warnings: 1,
      blocked: false,
    });
    expect(summarizeTools(toolReports('no-blender')).blocked).toBe(true);
  });
});
