import type { ReactNode } from 'react';
import { Button, Icon, Kbd } from '@maquette/ui';
import { AgentPrompt } from '../../AgentPrompt';

/** What the empty canvas's buttons add. */
export type EmptyAdd = 'note' | 'image' | 'link' | 'asset';

export interface CanvasEmptyProps {
  /** The canvas's file, "plans/combat-ideas.plan.json". */
  file: string;
  name: string;
  /** The camera plan, whose nodes are the stops of layout.json. */
  camera: boolean;
  onAdd: (kind: EmptyAdd) => void;
  /** The add keys and their names, as the bar shows them. */
  keys: ReactNode;
}

/** One way to start, with its button: the design's rows. */
function Way({ text, children }: { text: string; children?: ReactNode }) {
  return (
    <li className="flex min-h-9 items-center justify-between gap-4 border-t border-line py-1.5 first:border-t-0">
      <span className="text-fg-dim">{text}</span>
      {children ? <span className="flex flex-none gap-1.5">{children}</span> : null}
    </li>
  );
}

/**
 * An empty canvas: where it is saved, and the ways to start it, each with
 * its button (a note, a picture or a web page, assets of the Board, a
 * prompt for an agent). The camera plan has no stops instead: its panel's
 * Add stop starts the tour. The overlay lets a double click through to
 * write a note where it lands.
 */
export default function CanvasEmpty({ file, name, camera, onAdd, keys }: CanvasEmptyProps) {
  if (camera)
    return (
      <section aria-label="No camera stops yet" className="plan-empty">
        <div className="plan-empty-card">
          <span className="grid size-12 place-items-center rounded-[14px] border border-line bg-raised text-fg-dim">
            <Icon name="video-camera" size={22} />
          </span>
          <h2 className="text-panel-title">No camera stops yet</h2>
          <p className="max-w-[380px] text-fg-dim">
            layout.json has no camera stops. Add stop in the camera panel starts the tour, and Apply
            writes it to layout.json. An agent can propose a tour too.
          </p>
        </div>
      </section>
    );
  const prompt = `Fill the canvas "${name}" (${file}) in this Maquette project with notes and links that plan <what you want to plan>, using the Plan tools.`;
  return (
    <section aria-label="This canvas is empty" className="plan-empty">
      <div className="plan-empty-card">
        <span className="grid size-12 place-items-center rounded-[14px] border border-line bg-raised text-fg-dim">
          <Icon name="graph" size={22} />
        </span>
        <h2 className="text-panel-title">Add notes, pictures, links and assets</h2>
        <p className="max-w-[420px] text-fg-dim">
          This canvas is empty. It is saved as {file}, and kept saved as you go.
        </p>
        <ul className="flex w-full max-w-[460px] flex-col text-left">
          <Way text="Double click anywhere to write a note">
            <Button size="sm" icon="note-pencil" onClick={() => onAdd('note')}>
              Write a note
            </Button>
          </Way>
          <Way text="Add a picture or a web page">
            <Button size="sm" icon="image" onClick={() => onAdd('image')}>
              Picture
            </Button>
            <Button size="sm" icon="link-simple" onClick={() => onAdd('link')}>
              Web page
            </Button>
          </Way>
          <Way text="Bring in assets from the Board">
            <Button size="sm" icon="cube" onClick={() => onAdd('asset')}>
              Pick assets
            </Button>
          </Way>
        </ul>
        <AgentPrompt
          title="Or ask an agent to start it"
          prompt={prompt}
          hint="Agents use the Plan tools over MCP."
        />
        <p className="flex flex-wrap items-center justify-center gap-4 text-meta text-fg-mute">
          {keys}
          <span className="inline-flex items-center gap-1.5">
            <Kbd keys="Drag" /> pan
          </span>
        </p>
      </div>
    </section>
  );
}
