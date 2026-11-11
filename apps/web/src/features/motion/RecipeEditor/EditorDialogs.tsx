import { Icon } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import type { useRecipeEditor } from './RecipeEditor.hooks';

type Editor = ReturnType<typeof useRecipeEditor>;

/**
 * The open recipe's questions: save before leaving with unsaved edits, and
 * which side wins when an agent changed what the person edited.
 */
export function EditorDialogs({ v, file }: { v: Editor; file: string }) {
  return (
    <>
      {v.leave ? (
        <ChoiceDialog
          title="Save the recipe first?"
          onDismiss={v.leave.stay}
          choices={[
            { label: 'Leave without saving', variant: 'danger', onClick: v.leave.anyway },
            { label: 'Stay', onClick: v.leave.stay },
            {
              label: 'Save',
              variant: 'primary',
              loading: v.saving,
              disabled: Boolean(v.leave.blocked),
              disabledReason: v.leave.blocked,
              onClick: v.leave.save,
            },
          ]}
        >
          <p>{file} does not have your latest edits.</p>
          {v.saveError ? (
            <p className="mt-2 flex items-start gap-1.5 text-danger">
              <Icon name="warning-circle" size={14} className="mt-0.5 flex-none" />
              {v.saveError}
            </p>
          ) : null}
          <p className="mt-2">Save them, stay to keep editing, or leave and drop them.</p>
        </ChoiceDialog>
      ) : null}
      {v.outside?.asking && !v.leave ? (
        <ChoiceDialog
          title="The recipe changed outside"
          onDismiss={v.outside.later}
          choices={[
            { label: 'Use theirs', onClick: v.outside.useTheirs },
            { label: 'Keep mine', variant: 'primary', onClick: v.outside.keepMine },
          ]}
        >
          <p>
            An agent changed {file} while you had unsaved edits. Everything that does not clash is
            already merged.
          </p>
          <p className="mt-2">
            Both sides changed {v.outside.clashes.join(', ')}. Keep your edits there, or use the
            version on disk.
          </p>
        </ChoiceDialog>
      ) : null}
    </>
  );
}
