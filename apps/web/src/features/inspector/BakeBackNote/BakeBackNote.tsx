import { Button, InlineBanner } from '@maquette/ui';
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
  const { bake, from, busy, bakeNow, keepGrey } = useBakeBackNote(project, asset);
  if (!bake) return null;

  switch (bake.state) {
    case 'offered':
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
