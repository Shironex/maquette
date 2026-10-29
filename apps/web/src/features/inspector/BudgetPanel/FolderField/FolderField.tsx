import { TextField } from '@maquette/ui';
import { useFolderField } from './FolderField.hooks';

export interface FolderFieldProps {
  /** The asset's name, for the field's label. */
  name: string;
  /** The folder set for it, or null when its kind or imported file decides. */
  folder: string | null;
  /** Where it goes without one, shown as the placeholder. */
  fallback: string;
  disabled?: boolean;
  onSave: (folder: string | null) => void;
}

/** The folder an asset's model goes to in the game, typed and saved on Enter or leaving. */
export default function FolderField({
  name,
  folder,
  fallback,
  disabled,
  onSave,
}: FolderFieldProps) {
  const f = useFolderField(folder, onSave);
  return (
    <TextField
      aria-label={`Game folder of ${name}`}
      aria-invalid={f.problem ? true : undefined}
      title={f.problem ?? 'The folder inside the model folder this model ships to'}
      mono
      value={f.text}
      placeholder={fallback || 'top level'}
      disabled={disabled}
      onChange={e => f.setText(e.target.value)}
      onBlur={f.commit}
      onKeyDown={e => {
        if (e.key === 'Enter') f.commit();
      }}
      className="h-7"
      note={f.problem ? { tone: 'warning', text: f.problem } : undefined}
    />
  );
}
