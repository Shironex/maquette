/**
 * Clean background: Codex makes a copy of one ref with the object kept and
 * the clutter cleared, shown before and after. Using the copy puts it in
 * the ref's place with its view; the original moves to refs/originals.
 */
import { Button, Checkbox, CompareSlider, Icon, SegmentedControl } from '@maquette/ui';
import type { RefsView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { inProject } from '@/state/lab';
import { VIEW_LABEL } from '@/state/refs';
import { SectionTitle } from '../../SectionTitle';
import { useCleanBackground } from './CleanBackground.hooks';

export interface CleanBackgroundProps {
  view: RefsView;
  /** The ref it cleans. */
  file: string;
}

const GROUNDS = [
  { value: 'studio-grey', label: 'Studio grey' },
  { value: 'transparent', label: 'Transparent' },
] as const;

export default function CleanBackground({ view, file }: CleanBackgroundProps) {
  const c = useCleanBackground(view, file);
  const url = (path: string) => fileUrl(inProject(view.projectDir, path)) ?? '';
  const { copy, drawing, busy, tag } = c;

  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <section className="flex flex-col gap-2.5">
          <SectionTitle aside={`${VIEW_LABEL[tag]} ref`}>Before and after</SectionTitle>
          {copy ? (
            <CompareSlider
              before={url(file)}
              after={url(copy.file)}
              beforeLabel="Original"
              afterLabel="Clean copy"
              className="aspect-square w-full overflow-hidden rounded-card border border-line"
            />
          ) : (
            <div className="relative aspect-square overflow-hidden rounded-card border border-line bg-studio">
              <img src={url(file)} alt="The original" className="size-full object-contain" />
              {drawing ? (
                <span className="absolute inset-x-0 bottom-0 flex items-center justify-center gap-2 bg-panel/85 py-2 text-meta text-fg-dim">
                  <Icon name="circle-notch" spin size={14} /> Codex is cleaning it
                </span>
              ) : null}
            </div>
          )}
        </section>

        <section className="flex flex-col gap-2.5">
          <SectionTitle>The copy</SectionTitle>
          <div className="flex items-center justify-between gap-3">
            <span className="text-fg-dim">Background</span>
            <SegmentedControl
              label="Background"
              options={GROUNDS}
              value={c.ground}
              onChange={c.setGround}
            />
          </div>
          <Checkbox
            checked={c.pad}
            onChange={c.setPad}
            label="Center the object and pad to a square"
          />
          <Checkbox
            checked={c.extend}
            onChange={c.setExtend}
            label="Draw what the frame cut off (off keeps it true to the original)"
          />
          <p className="text-meta text-fg-mute">
            The object stays as it is; only the clutter around it goes. Using the copy moves the
            original to refs/originals, kept, and the copy stays {VIEW_LABEL[tag]}.
          </p>
        </section>
      </div>
      <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
        <span className="mr-auto text-meta text-fg-mute">1 Codex image, like the Lab</span>
        <Button variant="secondary" onClick={c.keepOriginal}>
          Keep the original
        </Button>
        {copy ? (
          <>
            <Button disabled={busy || drawing} onClick={c.draw}>
              Draw again
            </Button>
            <Button
              variant="primary"
              icon="check"
              loading={busy}
              disabled={busy}
              onClick={() => c.useCopy(copy.file)}
            >
              Use the clean copy
            </Button>
          </>
        ) : (
          <Button
            variant="primary"
            icon="magic-wand"
            loading={busy || drawing}
            disabled={busy || drawing}
            tooltip="Starts one Codex job; it spends Codex usage, like the Lab"
            onClick={c.draw}
          >
            Clean background
          </Button>
        )}
      </div>
    </>
  );
}
