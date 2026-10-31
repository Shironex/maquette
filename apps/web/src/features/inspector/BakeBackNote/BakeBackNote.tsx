import { Button, InlineBanner, TextField } from '@maquette/ui';
import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { MAX_RETEXTURE_PROMPT } from '@/state/split';
import { useBakeBackNote } from './BakeBackNote.hooks';

export interface BakeBackNoteProps {
  project: string;
  asset: string;
}

/**
 * Where the texture of the asset's newest split stands: a Tripo split that
 * lost it offers the bake back, a Meshy one bakes on its own, and a done
 * bake lets the untextured version be kept instead.
 */
export default function BakeBackNote({ project, asset }: BakeBackNoteProps) {
  const { bake, meshy, painting, retexture, from, busy, bakeNow, keepGrey } = useBakeBackNote(
    project,
    asset
  );
  if (!bake) return null;
  if (painting) {
    if (!['approval', 'queued', 'running'].includes(painting.state)) return null;
    return (
      <InlineBanner
        title={`Re-texturing v${bake.onto} on Meshy`}
        detail="It lands in the Inbox as the next version; the grey one stays."
      />
    );
  }

  switch (bake.state) {
    case 'offered':
      if (meshy) {
        return (
          <>
            <div className="flex flex-col gap-1.5 rounded-card border border-line bg-panel p-3">
              <h3 className="font-semibold">v{bake.onto} is grey for now</h3>
              <p className="text-meta text-fg-mute">
                Meshy drops the texture. Bake it back from {from} in Blender, free, which keeps the
                parts, or paint it again on Meshy, which may merge them into one mesh. v{bake.onto}{' '}
                stays either way.
              </p>
              <div className="flex gap-2">
                <Button className="flex-1" icon="palette" loading={busy} onClick={bakeNow}>
                  Bake back
                </Button>
                <Button className="flex-1" onClick={retexture.ask}>
                  Re-texture...
                </Button>
              </div>
            </div>
            {retexture.open ? (
              <ChoiceDialog
                title="Re-texture with Meshy?"
                onDismiss={retexture.keep}
                choices={[
                  { label: 'Cancel', onClick: retexture.keep },
                  {
                    label: 'Yes, re-texture',
                    variant: 'primary',
                    loading: retexture.sending,
                    disabled: Boolean(retexture.problem),
                    disabledReason: retexture.problem ?? undefined,
                    onClick: retexture.send,
                  },
                ]}
              >
                <div className="flex flex-col gap-2">
                  <p>
                    Paints v{bake.onto} again from your words. {retexture.cost}. You can cancel
                    while Meshy has not started. It lands in the Inbox as the next version, and the
                    Inbox says if Meshy merged the parts.
                  </p>
                  <TextField
                    aria-label="Texture prompt"
                    placeholder="Weathered iron, frosted glass, warm glow"
                    maxLength={MAX_RETEXTURE_PROMPT}
                    value={retexture.prompt}
                    onChange={e => retexture.setPrompt(e.target.value)}
                  />
                </div>
              </ChoiceDialog>
            ) : null}
          </>
        );
      }
      return (
        <InlineBanner
          tone="warning"
          title="Tripo did not keep the texture"
          detail={`Bake it back from ${from} onto v${bake.onto} in Blender, free. v${bake.onto} stays as it came.`}
          actions={
            <Button size="sm" icon="palette" loading={busy} onClick={bakeNow}>
              Bake back
            </Button>
          }
        />
      );
    case 'scheduled':
    case 'running':
      return (
        <InlineBanner
          title={`Baking the texture back from ${from}`}
          detail={`Headless in Blender, onto v${bake.onto}. It lands as the version after.`}
        />
      );
    case 'failed':
      return (
        <InlineBanner
          tone="danger"
          title="The bake back did not finish"
          detail={bake.message ?? undefined}
          actions={
            <Button size="sm" loading={busy} onClick={bakeNow}>
              Try again
            </Button>
          }
        />
      );
    case 'done':
      return (
        <InlineBanner
          tone="success"
          title={`Texture baked back as v${bake.version ?? ''}`}
          detail={`From ${from}. v${bake.onto} keeps the untextured parts, if you would rather have those.`}
          actions={
            <Button size="sm" variant="ghost" loading={busy} onClick={keepGrey}>
              Keep v{bake.onto}
            </Button>
          }
        />
      );
  }
}
