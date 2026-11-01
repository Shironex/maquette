import { STEP } from '@/state/onboarding';

/** Each step's title and lead; `{name}` is the first project's name. */
export const HEADINGS: Record<number, { title: string; lead: string }> = {
  [STEP.workspace]: {
    title: 'Pick a home for your projects',
    lead: 'I keep every project, the model inbox and the build cache under one root folder. Pick an empty folder, or an existing Maquette workspace to pick up where you left off.',
  },
  [STEP.toolchain]: {
    title: 'Check your tools',
    lead: 'I run these on your machine and found most of them already. Blender is the only one I cannot work without.',
  },
  [STEP.defaults]: {
    title: 'Set your defaults',
    lead: 'I use these for every new model and build. A project can override any of them later.',
  },
  [STEP.agents]: {
    title: 'Let your agents drive',
    lead: 'Maquette runs a small MCP server on this machine, so Claude Code and Codex can read the board, start pipeline steps and check budgets. It only listens on localhost.',
  },
  [STEP.project]: {
    title: 'Start your first project',
    lead: 'Start from a template, or import a folder you already have. I detect versioned .blend files and tools/ scripts when importing.',
  },
  [STEP.ready]: {
    title: '{name} is ready',
    lead: 'Here is what I set up. Open the project and I will walk you through the first run on the Board.',
  },
};
