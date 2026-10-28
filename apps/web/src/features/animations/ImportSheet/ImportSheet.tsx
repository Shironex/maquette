/**
 * Import clips: Mixamo FBX files or pack zips, read headless in Blender,
 * then shown file by file with what goes in, what is a duplicate and what
 * is left out. Nothing is added until Import.
 */
import { Button, InlineBanner, TextField, Toggle } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import { formatBytes } from '@/lib/format';
import ImportFileBlock from './ImportFileBlock';
import { useImportSheet } from './ImportSheet.hooks';

const LICENSE =
  'You may use these clips in your own games and sites, but not hand out the raw files. The library lives on this machine. Ship and Export never copy a raw clip; a clip leaves only baked into a model.';

const TAG =
  'rounded-pill border border-line px-1.5 font-mono text-[11px] leading-[18px] text-fg-dim';

export default function ImportSheet() {
  const s = useImportSheet();
  const { sheet, plan } = s;
  if (!sheet) return null;
  const files = plan?.files.map(f => (
    <ImportFileBlock
      key={f.name}
      file={f}
      choices={sheet.choices}
      onChoices={choices => s.change({ choices })}
    />
  ));
  const packFields = s.zips.map(z => (
    <TextField
      key={z.name}
      label={`Pack for ${z.name}`}
      value={sheet.packs[z.name] ?? z.pack}
      maxLength={80}
      onChange={e => s.change({ packs: { ...sheet.packs, [z.name]: e.target.value } })}
    />
  ));
  const tags = plan?.tags.map(t => (
    <span key={t.tag} className={TAG}>
      {t.tag} · {t.count}
    </span>
  ));
  const read = plan
    ? `${plan.files.length} ${plan.files.length === 1 ? 'file' : 'files'} · read headless in Blender · ${formatBytes(plan.readBytes ?? 0)} in`
    : 'FBX files or zips of them, from anywhere on this machine';

  return (
    <Sheet
      id="import-clips"
      icon="download-simple"
      title="Import clips"
      meta="Mixamo FBX files or a whole pack zip. Nothing is added until you import."
      onClose={s.close}
      width={560}
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
        <div className="flex items-center gap-3">
          <Button icon="plus" loading={sheet.reading} onClick={s.pickFiles}>
            Add files
          </Button>
          <span className="text-meta text-fg-mute">
            {sheet.reading ? 'Reading headless in Blender...' : read}
          </span>
        </div>
        {sheet.error ? (
          <InlineBanner tone="danger" title="Could not read the files" detail={sheet.error} />
        ) : null}
        {files?.length ? <ul className="flex list-none flex-col gap-2.5">{files}</ul> : null}
        {plan ? (
          <section aria-label="Into the library" className="flex flex-col gap-3">
            <h3 className="text-[11px] font-semibold tracking-[0.04em] text-fg-mute uppercase">
              Into the library
            </h3>
            {packFields}
            {s.singles ? (
              <TextField
                label="Single files go to"
                value={sheet.singles}
                maxLength={80}
                onChange={e => s.change({ singles: e.target.value })}
              />
            ) : null}
            {tags?.length ? (
              <div className="flex flex-col gap-1.5">
                <span className="text-meta text-fg-mute">Tags from the clip names</span>
                <p className="flex flex-wrap gap-1">{tags}</p>
              </div>
            ) : null}
            <dl className="grid grid-cols-2 gap-2 text-meta">
              <div>
                <dt className="text-fg-mute">FBX read</dt>
                <dd className="font-semibold">{formatBytes(plan.readBytes ?? 0)}</dd>
              </div>
              <div>
                <dt className="text-fg-mute">Stored, skeleton only</dt>
                <dd className="font-semibold">about {formatBytes(s.totals?.bytes ?? 0)}</dd>
              </div>
            </dl>
            <Toggle
              label="Keep the FBX files"
              description="Off: only the skeleton and keys are kept. On: each FBX is kept next to its clip."
              checked={sheet.keepFiles}
              onChange={keepFiles => s.change({ keepFiles })}
            />
          </section>
        ) : null}
        <InlineBanner
          tone="info"
          title="Mixamo license."
          detail={s.dir ? `${LICENSE} Clips go to ${s.dir}.` : LICENSE}
        />
      </div>
      <footer className="flex items-center gap-3 border-t border-line px-5 py-3.5">
        <span className="min-w-0 flex-1 text-meta text-fg-mute">
          <b className="text-[15px] font-semibold text-fg">{s.addLine}</b> to add
          {s.totals?.leftOut ? ` · ${s.totals.leftOut} left out` : ''}
        </span>
        <Button onClick={s.close}>Cancel</Button>
        <Button
          variant="primary"
          icon="download-simple"
          disabled={s.add === 0 || sheet.reading || s.busy}
          disabledReason={sheet.reading ? 'Still reading the files' : 'Add files with clips first'}
          loading={s.busy}
          onClick={s.run}
        >
          {s.importLabel}
        </Button>
      </footer>
    </Sheet>
  );
}
