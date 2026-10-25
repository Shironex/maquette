import { Icon, PathField, SegmentedControl, TextField, cn } from '@maquette/ui';
import { NAME_MAX, repoNote, type ProjectFields, type ProjectMode } from '@/state/onboarding';
import type { ProjectFieldActions } from '../useProjectFieldActions';
import { ImportPicker } from './ImportPicker';
import { TemplateGrid } from './TemplateGrid';
import { useProjectForm } from './ProjectForm.hooks';

const MODES = [
  { value: 'template', label: 'From a template', icon: 'squares-four' },
  { value: 'import', label: 'Import a folder', icon: 'folder-open' },
] as const;

export interface ProjectFormProps {
  value: ProjectFields;
  onChange: (patch: Partial<ProjectFields>) => void;
  actions: ProjectFieldActions;
  /** The project exists: only its name and site repo can change. */
  locked?: boolean;
  /** Smaller template cards and stacked fields, for the New project dialog. */
  compact?: boolean;
  /** Focus the name field first. */
  autoFocusName?: boolean;
}

/** A template or a folder to import, the name and the target site repo. */
export default function ProjectForm({
  value,
  onChange,
  actions,
  locked,
  compact,
  autoFocusName,
}: ProjectFormProps) {
  const repo = repoNote(value.repo);
  const { nameProblem, touchName } = useProjectForm(value.name);
  const size = compact ? 'md' : 'lg';

  return (
    <>
      {locked ? (
        <p className="mt-7 flex items-center gap-1.5 text-meta text-fg-mute">
          <Icon name="info" size={14} />
          The project exists. You can still rename it or change its site repo.
        </p>
      ) : (
        <>
          <SegmentedControl
            label="How to start"
            options={MODES}
            value={value.mode}
            onChange={(mode: ProjectMode) => onChange({ mode })}
            block
            className={compact ? 'w-full' : 'mt-7 w-[320px]'}
          />
          {value.mode === 'template' ? (
            <TemplateGrid
              value={value.template}
              compact={compact}
              onChange={template => onChange({ template })}
            />
          ) : (
            <ImportPicker
              folder={value.sourceFolder}
              size={size}
              onChange={sourceFolder => onChange({ sourceFolder })}
              onBrowse={() => void actions.browseSource()}
            />
          )}
        </>
      )}
      <div
        className={cn(
          'grid gap-3.5',
          compact ? 'mt-4 grid-cols-1' : 'mt-6 grid-cols-[minmax(0,0.8fr)_minmax(0,2fr)]'
        )}
      >
        <TextField
          label="Project name"
          size={size}
          value={value.name}
          maxLength={NAME_MAX}
          autoFocus={autoFocusName}
          onChange={e => onChange({ name: e.target.value })}
          onBlur={touchName}
          note={nameProblem ? { tone: 'missing', text: nameProblem } : undefined}
        />
        <PathField
          label="Target site repo"
          size={size}
          placeholder="Optional, a web repo Ship writes to"
          value={value.siteRepo}
          onChange={siteRepo => onChange({ siteRepo, repo: null })}
          onCommit={path => void actions.checkRepo(path)}
          onBrowse={() => void actions.browseRepo()}
        />
      </div>
      {repo ? (
        <p className="mt-2 flex items-center gap-1.5 text-meta text-fg-mute">
          <span
            className={cn(
              'inline-flex items-center gap-1.5',
              repo.tone === 'found'
                ? 'text-sage'
                : repo.tone === 'warning'
                  ? 'text-amber'
                  : 'text-danger'
            )}
          >
            <Icon
              name={
                repo.tone === 'found'
                  ? 'check-circle'
                  : repo.tone === 'warning'
                    ? 'warning'
                    : 'warning-circle'
              }
              size={14}
            />
            {repo.text}
          </span>
          {repo.detail}
        </p>
      ) : null}
    </>
  );
}
