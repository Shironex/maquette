import { Button, Checkbox, Select, TextField } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { SectionLabel } from '@/features/common/SectionLabel';
import { COMMERCIAL, CREDIT, MODIFICATION, PERMISSIONS, USAGE_FLAGS } from '@/state/avatar';
import { useMetaForm } from './MetaForm.hooks';

const text = (v: string | null | undefined) => v ?? '';
const orNull = (v: string) => (v.trim() ? v : null);

/** The VRM meta a build writes: who made the avatar and what others may do with it. */
export default function MetaForm() {
  const form = useMetaForm();
  const { meta, editMeta } = form;
  if (!meta) return null;
  const flags = USAGE_FLAGS.map(flag => (
    <Checkbox
      key={flag.key}
      label={flag.label}
      checked={meta[flag.key]}
      onChange={on => editMeta({ [flag.key]: on })}
    />
  ));
  return (
    <Panel
      id="avatar-meta"
      title="Meta"
      help={
        form.stored
          ? 'Saved with the project. A build saves changes first.'
          : 'From the workspace avatar defaults until the first save.'
      }
      actions={
        form.dirty ? (
          <Button
            size="sm"
            variant="secondary"
            icon="check"
            loading={form.saving}
            tooltip="Saves the meta and the material mode with the project"
            onClick={form.save}
          >
            Save
          </Button>
        ) : null
      }
    >
      <div className="grid grid-cols-2 gap-3">
        <TextField
          label="Name"
          value={meta.name}
          onChange={e => editMeta({ name: e.target.value })}
          note={
            form.problem && !meta.name.trim() ? { tone: 'warning', text: form.problem } : undefined
          }
        />
        <TextField
          label="Version"
          placeholder="Optional"
          value={text(meta.version)}
          onChange={e => editMeta({ version: orNull(e.target.value) })}
        />
        <TextField
          label="Authors"
          hint="Separate names with commas."
          value={form.authors}
          onChange={e => form.setAuthors(e.target.value)}
          note={
            form.problem && meta.name.trim() ? { tone: 'warning', text: form.problem } : undefined
          }
          fieldClassName="col-span-2"
        />
        <TextField
          label="Copyright"
          placeholder="Optional"
          value={text(meta.copyrightInformation)}
          onChange={e => editMeta({ copyrightInformation: orNull(e.target.value) })}
        />
        <TextField
          label="Contact"
          placeholder="Optional"
          value={text(meta.contactInformation)}
          onChange={e => editMeta({ contactInformation: orNull(e.target.value) })}
        />
      </div>
      <SectionLabel className="pt-1">Licence</SectionLabel>
      <div className="grid grid-cols-2 gap-3">
        <Select
          label="Who may play it"
          options={PERMISSIONS}
          value={meta.avatarPermission}
          onChange={v => editMeta({ avatarPermission: v })}
        />
        <Select
          label="Commercial use"
          options={COMMERCIAL}
          value={meta.commercialUsage}
          onChange={v => editMeta({ commercialUsage: v })}
        />
        <Select
          label="Credit"
          options={CREDIT}
          value={meta.creditNotation}
          onChange={v => editMeta({ creditNotation: v })}
        />
        <Select
          label="Changes"
          options={MODIFICATION}
          value={meta.modification}
          onChange={v => editMeta({ modification: v })}
        />
      </div>
      <div className="grid grid-cols-2 gap-x-3 gap-y-2">{flags}</div>
      <TextField
        label="Other licence URL"
        placeholder="Optional, on top of the VRM licence"
        mono
        value={text(meta.otherLicenseUrl)}
        onChange={e => editMeta({ otherLicenseUrl: orNull(e.target.value) })}
      />
    </Panel>
  );
}
