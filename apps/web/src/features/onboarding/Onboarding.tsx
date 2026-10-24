import type { ReactNode } from 'react';
import { Button, Icon, OnboardingStepper, WindowChrome, Wordmark } from '@maquette/ui';
import appIcon from '@/assets/app-icon.png';
import { CHROME } from '@/ipc/runtime';
import { STEP, STEPS } from '@/state/onboarding';
import { useOnboarding, type OnboardingApi } from './useOnboarding';
import { WelcomeStep } from './steps/WelcomeStep';
import { WorkspaceStep } from './steps/WorkspaceStep';
import { ToolchainStep, ToolchainFootNote } from './steps/ToolchainStep';
import { DefaultsStep } from './steps/DefaultsStep';
import { AgentsStep } from './steps/AgentsStep';
import { ProjectStep } from './steps/ProjectStep';
import { ReadyStep } from './steps/ReadyStep';

const BODY: Record<number, (api: OnboardingApi) => ReactNode> = {
  [STEP.workspace]: api => <WorkspaceStep api={api} />,
  [STEP.toolchain]: () => <ToolchainStep />,
  [STEP.defaults]: api => <DefaultsStep api={api} />,
  [STEP.agents]: api => <AgentsStep api={api} />,
  [STEP.project]: api => <ProjectStep api={api} />,
  [STEP.ready]: api => <ReadyStep api={api} />,
};

export function Onboarding() {
  const api = useOnboarding();
  const { step } = api;

  return (
    <WindowChrome title="Set up Maquette" mode={CHROME} className="h-full">
      <aside className="flex w-[296px] flex-none flex-col border-r border-line bg-panel px-6 pt-8 pb-6">
        <div className="mb-9 flex items-center gap-3">
          <img src={appIcon} alt="" className="size-10 rounded-[10px]" />
          <Wordmark size={18} />
        </div>
        <OnboardingStepper steps={STEPS} current={step} onSelect={i => api.setStep(i)} />
        <p className="mt-auto flex gap-2 text-meta leading-[17px] text-fg-mute">
          <Icon name="gear-six" size={14} className="mt-px" />
          Change any of this later in Settings.
        </p>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        {step === STEP.welcome ? (
          <WelcomeStep api={api} />
        ) : (
          <>
            <div className="min-h-0 flex-1 overflow-y-auto px-[72px] pt-14 pb-6">
              <div className="mx-auto w-full max-w-[760px]">
                <StepHeading step={step} name={api.draft.name} />
                {BODY[step]?.(api)}
              </div>
            </div>
            <footer className="flex h-[72px] flex-none items-center border-t border-line px-[72px]">
              <div className="mx-auto flex w-full max-w-[760px] items-center gap-2.5">
                <Button size="lg" onClick={api.back} disabled={api.flow.busy}>
                  Back
                </Button>
                {step === STEP.toolchain ? <ToolchainFootNote /> : null}
                <Button
                  size="lg"
                  variant="primary"
                  iconRight="arrow-right"
                  className="ml-auto"
                  disabled={!api.canContinue}
                  loading={api.flow.busy}
                  onClick={() => void api.next()}
                >
                  {api.continueLabel}
                </Button>
              </div>
            </footer>
          </>
        )}
      </main>
    </WindowChrome>
  );
}

const HEADINGS: Record<number, { title: string; lead: string }> = {
  [STEP.workspace]: {
    title: 'Pick a home for your projects',
    lead: 'I keep every project, the Tripo inbox and the build cache under one root folder. Pick an empty folder, or an existing Maquette workspace to pick up where you left off.',
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

function StepHeading({ step, name }: { step: number; name: string }) {
  const heading = HEADINGS[step];
  if (!heading) return null;
  return (
    <>
      <p className="font-mono text-[12px] leading-4 font-medium tracking-[0.02em] text-clay-text">
        Step {step + 1} of {STEPS.length}
      </p>
      <h1 className="mt-2.5 text-onboarding-title">
        {heading.title.replace('{name}', name.trim() || 'Your project')}
      </h1>
      <p className="mt-2.5 max-w-[600px] text-[15px] leading-[23px] text-fg-dim">{heading.lead}</p>
    </>
  );
}
