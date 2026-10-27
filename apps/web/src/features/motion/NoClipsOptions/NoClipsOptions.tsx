import { Button, Icon, type IconName } from '@maquette/ui';
import type { ReactNode } from 'react';
import { useNoClipsOptions } from './NoClipsOptions.hooks';

function Option({
  icon,
  title,
  body,
  action,
}: {
  icon: IconName;
  title: string;
  body: string;
  action: ReactNode;
}) {
  return (
    <div className="flex flex-1 flex-col gap-2 rounded-card border border-line bg-raised p-3.5">
      <Icon name={icon} size={18} className="text-fg-dim" />
      <p className="text-[13px] leading-[18px] font-semibold">{title}</p>
      <p className="text-meta text-fg-mute">{body}</p>
      <span className="mt-auto pt-1">{action}</span>
    </div>
  );
}

/** A model with no clips yet: the three ways a first one gets here. */
export default function NoClipsOptions() {
  const { model, retarget, openInbox, addScript } = useNoClipsOptions();
  if (!model) return null;
  const rigged = Boolean(model.rig);
  return (
    <section
      aria-label="No clips yet"
      className="flex flex-none flex-col gap-3.5 rounded-card border border-line bg-panel p-[18px]"
    >
      <div className="flex items-start gap-3">
        <Icon name="film-strip" size={18} className="mt-0.5 text-fg-mute" />
        <div>
          <h2 className="text-[15px] leading-5 font-semibold">No clips on {model.name} yet</h2>
          <p className="text-meta text-fg-mute">
            {rigged
              ? 'The rig is ready. Add a first clip, then play, trim and ship it from here.'
              : 'Clips that move the whole model or its parts play here once its file has them.'}
          </p>
        </div>
      </div>
      <div className="flex items-stretch gap-3">
        <Option
          icon="books"
          title="From the motion library"
          body={
            rigged
              ? `Map a walk, idle or gesture onto this rig. ${model.rig?.humanoid ? 'Its bones map onto the humanoid names library clips use.' : 'Some of its bones need a manual map.'}`
              : 'Library clips need a humanoid rig, and this model has none.'
          }
          action={
            <Button
              variant={rigged ? 'primary' : 'secondary'}
              icon="arrows-left-right"
              disabled={!rigged}
              disabledReason="Needs a rig"
              tooltip="Maps a library clip onto this rig"
              onClick={retarget}
            >
              Retarget a clip
            </Button>
          }
        />
        <Option
          icon="file"
          title="From a .glb or .blend"
          body="Clips you animated yourself: export them in the model and drop it in the Inbox as its next version. Every action keeps its name."
          action={
            <Button
              icon="tray"
              tooltip="Goes to the Inbox, where a model file can come in"
              onClick={openInbox}
            >
              Open the Inbox
            </Button>
          }
        />
        <Option
          icon="terminal-window"
          title="Bake with a Blender script"
          body="A script poses the rig and bakes the result headless, as a built asset on the Board."
          action={
            <Button
              icon="plus"
              tooltip="Adds a built asset on the Board that a Blender script makes"
              onClick={addScript}
            >
              Add script
            </Button>
          }
        />
      </div>
    </section>
  );
}
