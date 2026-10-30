import { Button, CheckRow, Icon, KeyValueList, SegmentedControl } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { SectionLabel } from '@/features/common/SectionLabel';
import { MATERIALS } from '@/state/avatar';
import { HIGHLIGHTS } from '@/state/avatar-highlights';
import { useExportPanel } from './ExportPanel.hooks';

/** How the VRM shades, its baked highlights, where it goes, and the file the last build wrote. */
export default function ExportPanel() {
  const p = useExportPanel();
  if (!p.view) return null;
  const { written } = p;
  const unresolved = (written?.unresolved ?? []).map(f => (
    <CheckRow
      key={f.id}
      status={f.level === 'blocker' ? 'fail' : f.level === 'warning' ? 'warning' : 'pass'}
      title={f.title}
      detail={f.detail ?? undefined}
    />
  ));
  return (
    <Panel
      id="avatar-export"
      title="Export"
      help="VRM 1.0, written to the project's exports folder. The model's versions stay as they are."
    >
      <div className="flex flex-col gap-1.5">
        <SegmentedControl
          label="Material"
          tone="clay"
          options={MATERIALS.map(m => ({ value: m.value, label: m.label, hint: m.hint }))}
          value={p.material}
          onChange={p.setMaterial}
        />
        <p className="text-meta text-fg-mute">{p.hint}</p>
      </div>
      {p.toon ? (
        <div className="flex flex-col gap-1.5">
          <SegmentedControl
            label="Baked highlights"
            tone="clay"
            options={HIGHLIGHTS.map(h => ({ value: h.value, label: h.label, hint: h.hint }))}
            value={p.highlights}
            onChange={p.setHighlights}
          />
          <p className="text-meta text-fg-mute">{p.highlightsHint}</p>
        </div>
      ) : null}
      <div className="flex flex-col gap-1">
        <SectionLabel>Output</SectionLabel>
        <p className="truncate font-mono text-[12px] text-fg-dim" title={p.view.exportPath}>
          {p.view.exportPath}
        </p>
      </div>
      {written ? (
        <section
          aria-label="Last export"
          className="flex flex-col gap-3 border-t border-line pt-3.5"
        >
          <div className="flex items-center justify-between gap-2">
            <SectionLabel>
              {written.exists === false ? 'Last export, file gone' : 'Last export'}
            </SectionLabel>
            <div className="flex gap-1.5">
              <Button
                size="sm"
                variant="secondary"
                icon="folder-open"
                disabled={written.exists === false}
                tooltip="Shows the file in the file manager"
                onClick={p.reveal}
              >
                Reveal
              </Button>
              <Button
                size="sm"
                variant="secondary"
                icon="copy"
                tooltip="Copies the file's full path"
                onClick={p.copy}
              >
                Copy path
              </Button>
            </div>
          </div>
          <KeyValueList items={p.facts} columns={3} />
          {p.roundTrip ? (
            <p
              className={`flex items-center gap-1.5 text-meta ${p.roundTrip.passed ? 'text-sage' : 'text-danger'}`}
            >
              <Icon name={p.roundTrip.passed ? 'check-circle' : 'x-circle'} size={14} />
              Round trip: {p.roundTrip.text}
            </p>
          ) : null}
          {p.report ? (
            <div className="flex items-center justify-between gap-2">
              <div className="flex min-w-0 flex-col gap-0.5">
                <SectionLabel>Report</SectionLabel>
                <p className="truncate font-mono text-[12px] text-fg-dim" title={p.report}>
                  {p.report}
                </p>
              </div>
              <Button
                size="sm"
                variant="secondary"
                icon="file-text"
                tooltip="Shows the validation report in the file manager"
                onClick={p.revealReport}
              >
                Reveal report
              </Button>
            </div>
          ) : null}
          {unresolved.length > 0 ? (
            <div className="flex flex-col gap-2">
              <SectionLabel>Still in the file · {unresolved.length}</SectionLabel>
              {unresolved}
            </div>
          ) : (
            <p className="text-meta text-sage">Nothing left to fix.</p>
          )}
        </section>
      ) : (
        <p className="text-meta text-fg-mute">No VRM written yet.</p>
      )}
    </Panel>
  );
}
