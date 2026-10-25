import { TextArea } from '@maquette/ui';
import { MAX_PROMPT } from '@/state/providers';

export interface PromptFieldProps {
  value: string;
  onChange: (prompt: string) => void;
  /** Names the field after its asset in a batch. */
  name?: string;
  rows?: number;
}

/** Meshy's texture prompt: a few words that steer the texture, at most 800 characters. */
export default function PromptField({ value, onChange, name, rows = 3 }: PromptFieldProps) {
  const over = value.length > MAX_PROMPT;
  return (
    <TextArea
      label={name ? `Texture prompt for ${name}` : 'Texture prompt'}
      hint="Optional. Meshy textures the model with these words as a guide."
      rows={rows}
      value={value}
      placeholder="Weathered teal paint, rust at the edges"
      onChange={e => onChange(e.target.value)}
      note={
        over
          ? {
              tone: 'warning',
              text: `${value.length} of ${MAX_PROMPT} characters. Meshy refuses longer ones.`,
            }
          : { tone: 'found', text: `${value.length} of ${MAX_PROMPT} characters` }
      }
    />
  );
}
