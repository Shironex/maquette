import { Button } from '@maquette/ui';
import { RecipeBake } from '../RecipeBake';
import type { useEditorApply } from './RecipeEditor.hooks';

/** Apply, or Save and apply with unsaved edits. */
export function ApplyButton({ v }: { v: ReturnType<typeof useEditorApply> }) {
  return (
    <Button
      size="sm"
      icon="play"
      loading={v.loading}
      disabled={Boolean(v.blocked)}
      disabledReason={v.blocked}
      tooltip={v.bake.model ? `Bakes it onto ${v.bake.model.name}` : undefined}
      onClick={v.apply}
    >
      {v.dirty ? 'Save and apply' : 'Apply'}
    </Button>
  );
}

/** The open recipe's last bake. */
export function EditorBake({ v }: { v: ReturnType<typeof useEditorApply> }) {
  return (
    <RecipeBake
      record={v.bake.record}
      assetName={v.bake.assetName}
      busy={v.bake.busy}
      error={v.bake.error}
      stale={v.bake.stale}
      onCancel={v.bake.cancel}
    />
  );
}
