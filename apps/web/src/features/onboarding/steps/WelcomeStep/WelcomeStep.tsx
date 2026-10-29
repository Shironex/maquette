import { Fragment } from 'react';
import { Button, Icon, PIPELINE_STAGES } from '@maquette/ui';
import appIcon from '@/assets/app-icon.png';
import welcome from '@/assets/welcome.jpg';
import type { WelcomeStepProps } from './WelcomeStep.types';

const stages = PIPELINE_STAGES.map((stage, i) => (
  <Fragment key={stage}>
    {i > 0 ? <Icon name="caret-right" size={12} /> : null}
    <span className="text-fg-dim">{stage}</span>
  </Fragment>
));

export default function WelcomeStep({ api }: WelcomeStepProps) {
  return (
    <div className="flex min-h-0 flex-1 items-center gap-14 overflow-y-auto px-[72px]">
      <div className="flex w-[440px] flex-none flex-col">
        <img
          src={appIcon}
          alt="Maquette app icon, a clay figure on a plaster plinth"
          className="size-[104px] rounded-[24px] shadow-[0_18px_40px_-18px_rgba(10,9,8,0.9)]"
        />
        <h1 className="mt-7 text-[48px] leading-[52px] font-semibold tracking-[-0.045em]">
          maquette
        </h1>
        <p className="mt-3 text-[18px] leading-[27px] text-fg-dim">
          Turn concept art into real-time 3D scenes.
        </p>
        <div
          className="mt-[26px] flex flex-wrap items-center gap-1.5 text-[12.5px] text-fg-mute"
          aria-label="Pipeline: Concept, Refs, 3D model, Blender, Web, Ship"
        >
          {stages}
        </div>
        <div className="mt-9 flex gap-2.5">
          <Button
            size="lg"
            variant="primary"
            iconRight="arrow-right"
            onClick={() => void api.next()}
          >
            Set up
          </Button>
          <Button
            size="lg"
            icon="folder-open"
            disabled={api.flow.busy}
            tooltip="Skips setup and opens a workspace folder you already have"
            onClick={() => void api.openExistingFromWelcome()}
          >
            Open existing workspace
          </Button>
        </div>
        <p className="mt-3.5 text-meta text-fg-mute">
          Setup takes about two minutes. Nothing leaves this machine.
        </p>
      </div>
      <figure className="m-0 flex min-w-0 flex-1 flex-col gap-2.5">
        <div className="aspect-video overflow-hidden rounded-[16px] border border-line shadow-[0_30px_60px_-30px_rgba(10,9,8,0.9)]">
          <img
            src={welcome}
            alt="Elsheria on the frozen rune lake at night, from the game Frostveil"
            className="block size-full object-cover"
          />
        </div>
        <figcaption className="text-meta text-fg-mute">
          Frostveil: its props and characters came through this pipeline.
        </figcaption>
      </figure>
    </div>
  );
}
