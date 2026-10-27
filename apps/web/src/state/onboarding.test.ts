import { describe, expect, it } from 'vitest';
import type { RootInspection } from '@/ipc/client';
import { defaultConfig, providerCatalog, toolReports } from '@/ipc/mock/fixtures';
import { patchSection } from './settings-form';
import {
  STEP,
  canContinue,
  continueLabel,
  initialDraft,
  nextStep,
  readySummary,
  rootNote,
  type FlowState,
} from './onboarding';

function inspection(state: RootInspection['state']): RootInspection {
  return { path: '/Users/maker/Maquette', display: '~/Maquette', state, entries: 0 };
}

function flow(patch: Partial<FlowState> = {}, draft: Partial<FlowState['draft']> = {}): FlowState {
  return {
    draft: { ...initialDraft('~/Maquette', defaultConfig()), ...draft },
    tools: toolReports('first-run'),
    checking: false,
    created: false,
    busy: false,
    ...patch,
  };
}

describe('onboarding steps', () => {
  it('only lets the workspace step continue with a usable folder', () => {
    expect(canContinue(STEP.workspace, flow())).toBe(false);
    expect(canContinue(STEP.workspace, flow({}, { inspection: inspection('empty') }))).toBe(true);
    expect(canContinue(STEP.workspace, flow({}, { inspection: inspection('missing') }))).toBe(true);
    expect(canContinue(STEP.workspace, flow({}, { inspection: inspection('not-empty') }))).toBe(
      false
    );
    expect(rootNote(flow({}, { inspection: inspection('not-empty') }).draft, false)?.tone).toBe(
      'missing'
    );
  });

  it('offers to open an existing workspace instead of continuing', () => {
    const f = flow({}, { inspection: inspection('workspace') });
    expect(continueLabel(STEP.workspace, f)).toBe('Open workspace');
    expect(continueLabel(STEP.workspace, { ...f, created: true })).toBe('Save and review');
  });

  it('blocks the toolchain step only while checking or without Blender', () => {
    expect(canContinue(STEP.toolchain, flow())).toBe(true);
    expect(canContinue(STEP.toolchain, flow({ checking: true }))).toBe(false);
    expect(canContinue(STEP.toolchain, flow({ tools: toolReports('no-blender') }))).toBe(false);
    expect(canContinue(STEP.toolchain, flow({ tools: [] }))).toBe(false);
  });

  it('blocks defaults and agents on an invalid config', () => {
    const config = patchSection(defaultConfig(), 'agents', { mcpPort: 12 });
    expect(canContinue(STEP.defaults, flow({}, { config }))).toBe(false);
    expect(canContinue(STEP.agents, flow({}, { config }))).toBe(false);
  });

  it('needs a name, and a folder when importing', () => {
    expect(canContinue(STEP.project, flow())).toBe(true);
    expect(canContinue(STEP.project, flow({}, { name: '   ' }))).toBe(false);
    expect(canContinue(STEP.project, flow({}, { mode: 'import' }))).toBe(false);
    expect(canContinue(STEP.project, flow({}, { mode: 'import', sourceFolder: '~/old' }))).toBe(
      true
    );
    expect(continueLabel(STEP.project, flow())).toBe('Create project');
    expect(continueLabel(STEP.ready, flow())).toBe('Open Frostveil');
  });

  it('returns to Ready after editing a step once things exist', () => {
    expect(nextStep(STEP.defaults, false)).toBe(STEP.agents);
    expect(nextStep(STEP.defaults, true)).toBe(STEP.ready);
    expect(nextStep(STEP.welcome, true)).toBe(STEP.workspace);
  });

  it('summarizes the setup for the Ready step', () => {
    const rows = readySummary(
      flow({}, { inspection: inspection('empty'), siteRepo: '~/Documents/Projects/frostveil-site' })
        .draft,
      toolReports('first-run'),
      providerCatalog()
    );
    const get = (id: string) => rows.find(r => r.id === id)!;
    expect(get('workspace')).toMatchObject({
      mono: '~/Maquette',
      text: 'with projects/, inbox/ and cache/',
    });
    expect(get('toolchain')).toMatchObject({
      text: '5 of 6 found.',
      warning: 'toktx missing',
      effect: 'textures stay PNG until it is installed',
    });
    expect(get('defaults').text).toBe(
      'Tripo H3.1, Meshy 7.1. Characters: 50k faces, multi-view, rig with Mixamo. 3.0\u00a0MB scene, 60k\u00a0tris, 2K, ETC1S, meshopt medium'
    );
    expect(get('agents').text).toBe(
      'MCP server off. Headless Blender, no screen capture, ask before shipping'
    );
    expect(get('project').text).toBe(
      'Frostveil, Floating diorama, ships to Projects/frostveil-site'
    );
    expect(canContinue(STEP.ready, flow({ busy: true }))).toBe(false);
  });
});
