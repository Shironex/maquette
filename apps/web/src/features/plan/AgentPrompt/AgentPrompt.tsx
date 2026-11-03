import { Button, Icon } from '@maquette/ui';
import { useAgentPrompt } from './AgentPrompt.hooks';

export interface AgentPromptProps {
  /** What the box offers, "Or let an agent start one". */
  title: string;
  /** The prompt to paste into an agent; angle brackets mark what to fill in. */
  prompt: string;
  /** One line on how agents reach the canvas. */
  hint: string;
}

/**
 * A prompt to start a canvas with an agent, with Copy prompt: the empty
 * Plan list and an empty canvas offer it beside doing it by hand.
 */
export default function AgentPrompt({ title, prompt, hint }: AgentPromptProps) {
  const v = useAgentPrompt(prompt);
  return (
    <section
      aria-label={title}
      className="flex w-full max-w-[460px] flex-col gap-2 rounded-card border border-line bg-raised p-3 text-left"
    >
      <span className="flex items-center gap-2 text-[12.5px] font-medium text-fg-dim">
        <Icon name="robot" size={14} />
        {title}
      </span>
      <p className="rounded-[8px] bg-app px-2.5 py-2 font-mono text-[12px] leading-[18px] text-fg">
        {prompt}
      </p>
      <span className="flex items-center justify-between gap-3">
        <span className="text-meta text-fg-mute">{hint}</span>
        <Button size="sm" icon="copy" onClick={v.copy}>
          Copy prompt
        </Button>
      </span>
      <span role="status" className="sr-only">
        {v.copied === 'copied'
          ? 'Prompt copied'
          : v.copied === 'failed'
            ? 'Could not copy, the clipboard is not available here'
            : ''}
      </span>
    </section>
  );
}
