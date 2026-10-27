import { Button } from '@maquette/ui';
import appIcon from '@/assets/app-icon.png';
import { useFirstRunCard } from './FirstRunCard.hooks';

export interface FirstRunCardProps {
  /** The empty project's name. */
  name: string;
  /** The scene budget in MB; a game has none. */
  budget: number;
}

/** The tips over an empty Board, until they are hidden. */
export default function FirstRunCard({ name, budget }: FirstRunCardProps) {
  const { hidden, hide, setView, steps: tips, importModels } = useFirstRunCard();
  if (hidden) return null;
  const steps = tips.map((step, i) => (
    <li key={step.title} className="flex gap-3.5 border-t border-line py-3">
      <span className="grid size-7 flex-none place-items-center rounded-full border-[1.5px] border-clay-line font-mono text-[12px] leading-none font-semibold text-clay-text">
        {i + 1}
      </span>
      <div>
        <div className="text-[14px] font-semibold">{step.title}</div>
        <div className="mt-[3px] text-[12.5px] leading-[18px] text-fg-mute">
          {step.detail.replace('{budget}', budget.toFixed(1))}
        </div>
      </div>
    </li>
  ));
  return (
    <section
      aria-labelledby="first-run"
      className="absolute top-1/2 left-1/2 w-[560px] max-w-[calc(100%-32px)] -translate-x-1/2 -translate-y-[46%] rounded-[14px] border border-line-strong bg-panel px-7 pt-7 pb-6 shadow-window"
    >
      <div className="flex items-center gap-3.5">
        <img src={appIcon} alt="" className="size-12 rounded-[12px]" />
        <div>
          <h2 id="first-run" className="text-panel-title">
            Your first build in three steps
          </h2>
          <p className="mt-0.5 text-fg-dim">
            {name} is empty. Start with a concept and I will carry it through.
          </p>
        </div>
      </div>
      <ol className="mt-5 list-none p-0">{steps}</ol>
      <div className="mt-[18px] flex items-center gap-2.5 border-t border-line pt-[18px]">
        <Button
          size="lg"
          variant="primary"
          icon="flask"
          tooltip="Concept lab: make concept art for the project's assets"
          onClick={() => setView('lab')}
        >
          Open Concept lab
        </Button>
        <Button
          size="lg"
          icon="download-simple"
          tooltip="Import models: bring existing models into this project"
          onClick={importModels}
        >
          Import models
        </Button>
        <Button
          variant="ghost"
          className="ml-auto"
          tooltip="Hides this card; the Board stays as it is"
          onClick={hide}
        >
          Hide tips
        </Button>
      </div>
    </section>
  );
}
