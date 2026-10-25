import { Button, Icon, InlineBanner, Tooltip } from '@maquette/ui';
import { FIXUP_HINT, FIXUP_LABEL, withFixup } from '@/state/prepare';
import { SectionLabel } from '../SectionLabel';
import { FixupRow } from './FixupRow';
import { usePrepEditor } from './PrepEditor.hooks';
import type { PrepEditorProps } from './PrepEditor.types';
import { PreviewNumbers } from './PreviewNumbers';
import { SizeRow } from './SizeRow';

/**
 * An asset's real size and fix-ups: the size its model is scaled to and the
 * repairs made on a copy before the scene is built, with a headless preview
 * of the numbers before and after.
 */
export default function PrepEditor(props: PrepEditorProps) {
  const { value, onChange, preview, onPreview, previewBlocked, disabled } = props;
  const editor = usePrepEditor(props);
  const fixups = value.fixups.map((fixup, i) => (
    <FixupRow
      key={fixup.op}
      fixup={fixup}
      index={i + 1}
      disabled={disabled}
      onChange={next => onChange(withFixup(value, next))}
      onRemove={() => editor.remove(fixup.op)}
    />
  ));
  const adds = editor.missing.map(kind => (
    <Tooltip key={kind} label={FIXUP_HINT[kind]} wrap>
      <Button
        size="sm"
        variant="ghost"
        icon="plus"
        disabled={disabled}
        onClick={() => editor.add(kind)}
      >
        {FIXUP_LABEL[kind]}
      </Button>
    </Tooltip>
  ));
  const running = preview?.status === 'running';

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        <div className="flex items-baseline justify-between gap-2">
          <SectionLabel>Real size</SectionLabel>
          {editor.measuredText ? (
            <span className="text-meta text-fg-mute">Measured {editor.measuredText}</span>
          ) : null}
        </div>
        {value.size ? (
          <SizeRow
            size={value.size}
            disabled={disabled}
            onChange={size => onChange({ ...value, size })}
            onClear={editor.clearSize}
          />
        ) : (
          <Button
            size="sm"
            icon="ruler"
            disabled={disabled}
            onClick={editor.startSize}
            className="self-start"
          >
            Set real size
          </Button>
        )}
        {editor.scaledText ? (
          <p className="text-meta text-fg-mute">Scaled {editor.scaledText}</p>
        ) : null}
        {editor.flat ? (
          <p className="text-meta text-amber">
            That side of the model is flat, so it cannot scale by it.
          </p>
        ) : null}
        {editor.suggested ? (
          <button
            type="button"
            disabled={disabled}
            onClick={editor.suggested.use}
            className="flex cursor-pointer items-center gap-1.5 self-start rounded-pill border border-dashed border-line-strong px-2.5 py-1 text-meta text-fg-dim hover:bg-hover hover:text-fg disabled:cursor-default"
            title={`A ${editor.suggested.matched} is usually about this size. It applies only when you pick it.`}
          >
            <Icon name="sparkle" size={12} className="text-clay-text" />
            Suggested: {editor.suggested.text}
          </button>
        ) : null}
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex items-baseline justify-between gap-2">
          <SectionLabel>Fix-ups</SectionLabel>
          {fixups.length ? <span className="text-meta text-fg-mute">in this order</span> : null}
        </div>
        {fixups.length ? (
          <ol className="flex flex-col gap-1.5">{fixups}</ol>
        ) : (
          <p className="text-meta text-fg-mute">
            None. They run headless on a copy; the model file never changes.
          </p>
        )}
        {adds.length ? <div className="-ml-1.5 flex flex-wrap gap-0.5">{adds}</div> : null}
      </div>

      {onPreview ? (
        <div className="flex flex-col gap-2">
          <Button
            size="sm"
            icon="play"
            loading={running}
            disabled={disabled || editor.empty || running || Boolean(previewBlocked)}
            disabledReason={
              previewBlocked ?? (editor.empty ? 'Set a real size or add a fix-up first' : undefined)
            }
            tooltip="Runs them headless on a copy and shows the numbers before and after"
            onClick={onPreview}
            className="self-start"
          >
            {running ? 'Running the fix-ups' : 'Preview the numbers'}
          </Button>
          {preview?.status === 'done' ? <PreviewNumbers report={preview.report} /> : null}
          {preview?.status === 'failed' ? (
            <InlineBanner tone="danger" title="The preview did not run" detail={preview.error} />
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
