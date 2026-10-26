import { Button, CheckRow, KeyValueList } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { useRigCheck } from './RigCheck.hooks';

/** The rig check of the selected model: bones, weights and what the library misses. */
export default function RigCheck() {
  const { model, lines, file, openMap, facts } = useRigCheck();
  if (!model) return null;
  const rows = lines.map(line => (
    <CheckRow
      key={line.id}
      status={line.status}
      title={line.title}
      detail={line.detail}
      footer={
        line.map ? (
          <Button size="sm" icon="bone" onClick={openMap}>
            Map bones
          </Button>
        ) : undefined
      }
    />
  ));
  return (
    <>
      <section aria-label="Rig check" className="flex flex-col gap-2 border-t border-line pt-3.5">
        <div className="flex items-center justify-between gap-2">
          <SectionLabel>Rig check</SectionLabel>
          <span className="truncate font-mono text-[11.5px] text-fg-mute">{file}</span>
        </div>
        {rows}
      </section>
      <section aria-label="Model file" className="flex flex-col gap-3 border-t border-line pt-3.5">
        <SectionLabel>Model file</SectionLabel>
        <KeyValueList
          items={facts}
          columns={1}
          className="[&>div]:flex-row [&>div]:justify-between"
        />
      </section>
    </>
  );
}
