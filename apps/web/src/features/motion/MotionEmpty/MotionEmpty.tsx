import { Button, Icon } from '@maquette/ui';
import { useMotionEmpty } from './MotionEmpty.hooks';

const STEPS = [
  {
    title: 'Bring in a rigged model',
    detail:
      'Send a character to 3D with rigging on. The Inbox checks its bones before it reaches Blender.',
  },
  {
    title: 'Or give a prop states',
    detail:
      'Off and on, closed and open, at rest and swinging. Each state is a pose of its parts; a transition times the move between two.',
  },
  {
    title: 'Ship it with the scene',
    detail:
      'Clips ship inside the models. States are saved in layout.json next to the camera stops.',
  },
];

/** A game has no states: its models move by their own clips. */
const GAME_STEPS = [
  {
    title: 'Import a rigged model',
    detail:
      "Import models on the Board brings in the game's characters with their bones and clips.",
  },
  {
    title: 'Or send a character to 3D with rigging on',
    detail: 'The Inbox checks its bones before it reaches Blender.',
  },
  {
    title: 'Ship it to the game',
    detail: 'Game export keeps the rig and every clip in the model file.',
  },
];

/** Nothing in the project moves yet: what makes a model show here. */
export default function MotionEmpty() {
  const { name, count, suggestion, game } = useMotionEmpty();
  const steps = (game ? GAME_STEPS : STEPS).map((step, i) => (
    <li key={step.title} className="flex items-start gap-3 border-t border-line py-3">
      <span className="grid size-6 flex-none place-items-center rounded-full border-[1.5px] border-line-strong font-mono text-[11.5px] font-semibold text-fg-dim">
        {i + 1}
      </span>
      <div>
        <p className="font-semibold">{step.title}</p>
        <p className="mt-0.5 text-meta text-fg-mute">{step.detail}</p>
      </div>
    </li>
  ));
  return (
    <section aria-label="No motion yet" className="flex min-w-0 flex-1 overflow-y-auto px-5 py-4">
      <div className="mx-auto w-full max-w-[560px] pt-[72px]">
        <Icon name="person-simple-walk" size={28} className="text-fg-dim" />
        <h1 className="mt-3.5 text-screen-title">Nothing in {name} moves yet</h1>
        <p className="mt-1.5 text-[14px] leading-[21px] text-fg-dim">
          Motion lists the models that carry a rig, clips{game ? '' : ' or states'}. {name} has{' '}
          {count} {count === 1 ? 'model' : 'models'} and none of them do.
        </p>
        <ol className="mt-[22px]">{steps}</ol>
        {suggestion ? (
          <div className="mt-[18px] flex items-center gap-3.5 rounded-card border border-line bg-raised p-3.5">
            <span className="grid size-[72px] flex-none place-items-center overflow-hidden rounded-[8px] bg-studio">
              {suggestion.thumb ? (
                <img src={suggestion.thumb} alt="" className="size-full object-cover" />
              ) : (
                <Icon name="cube" size={24} className="text-fg-mute" />
              )}
            </span>
            <div className="min-w-0 flex-1">
              <p className="text-[13px] leading-[18px] font-semibold">
                {suggestion.model.name} could move
              </p>
              <p className="text-meta text-fg-mute">
                {suggestion.model.parts.length} separate parts, each with its own pivot. A good
                first prop with states: at rest and moved.
              </p>
            </div>
            <Button variant="primary" icon="plus" onClick={suggestion.give}>
              Give it states
            </Button>
          </div>
        ) : null}
      </div>
    </section>
  );
}
