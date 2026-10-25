import { Button, TextArea, TextField } from '@maquette/ui';
import { MAX_NOTE_CHARS, MAX_OBJECT_CHARS } from '@/state/lab';
import { useObjectEditor } from './ObjectEditor.hooks';

export interface ObjectEditorProps {
  name: string;
  notes: string;
  onSave: (next: { name: string; notes: string }) => boolean;
  onCancel: () => void;
}

/** Name and notes of one object. The name becomes the asset; the notes only go to Codex. */
export default function ObjectEditor({ name, notes, onSave, onCancel }: ObjectEditorProps) {
  const {
    draftName,
    rename,
    draftNotes,
    setDraftNotes,
    error,
    tooLong,
    save,
    onGroupKeyDown,
    onNameKeyDown,
  } = useObjectEditor(name, notes, onSave);
  return (
    <div
      role="group"
      aria-label={`Edit ${name}`}
      className="flex flex-col gap-3 rounded-card border border-line bg-raised p-3"
      onKeyDown={onGroupKeyDown}
    >
      <TextField
        label="Name"
        autoFocus
        value={draftName}
        onChange={e => rename(e.target.value)}
        onKeyDown={onNameKeyDown}
        hint={`${draftName.trim().length} of ${MAX_OBJECT_CHARS} characters. It names the asset.`}
        note={
          tooLong
            ? { tone: 'missing', text: `Keep the name to ${MAX_OBJECT_CHARS} characters.` }
            : error
              ? { tone: 'missing', text: error }
              : undefined
        }
      />
      <TextArea
        label="Notes for Codex"
        rows={3}
        maxLength={MAX_NOTE_CHARS}
        value={draftNotes}
        placeholder="Painted stone and moss, no wireframe lines, nothing floating around it"
        onChange={e => setDraftNotes(e.target.value)}
        hint={`${draftNotes.length} of ${MAX_NOTE_CHARS} characters. Optional, not part of the name.`}
      />
      <div className="flex justify-end gap-2">
        <Button variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button variant="primary" disabled={tooLong} onClick={save}>
          Done
        </Button>
      </div>
    </div>
  );
}
