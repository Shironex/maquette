import { Icon, PathField, SegmentedControl, TextField, cn } from '@maquette/ui';
import type { ProjectTarget } from '@/ipc/client';
import { NAME_MAX, type ProjectFields, type ProjectMode } from '@/state/onboarding';
import { gameRepoLines, siteRepoLines } from '@/state/repo-lines';
import type { ProjectFieldActions } from '../useProjectFieldActions';
import { GameFields } from './GameFields';
import { ImportPicker } from './ImportPicker';
import { RepoLines } from './RepoLines';
import { TargetPicker } from './TargetPicker';
import { GameTemplatePicker } from './GameTemplatePicker';
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
  /** The project exists: only its name and repo can change. */
  locked?: boolean;
  /** Smaller template cards and stacked fields, for the New project dialog. */
  compact?: boolean;
  /** Focus the name field first. */
  autoFocusName?: boolean;
}

/** Where it ships, a template or a folder to import, the name and the repo Ship writes to. */
export default function ProjectForm({
  value,
  onChange,
  actions,
  locked,
  compact,
  autoFocusName,
}: ProjectFormProps) {
  const game = value.target === 'game';
  const lines = game
    ? gameRepoLines(value.game, value.gameFormat, value.modelsDir)
    : siteRepoLines(value.repo);
  const { nameProblem, touchName } = useProjectForm(value.name);
  const size = compact ? 'md' : 'lg';
  const pickTarget = (target: ProjectTarget) => {
    if (target === value.target) return;
    onChange({ target });
    if (value.siteRepo.trim()) void actions.checkRepo(value.siteRepo, target);
  };

  return (
    <>
      {locked ? (
        <p className="mt-7 flex items-center gap-1.5 text-meta text-fg-mute">
          <Icon name="info" size={14} />
          The project exists. You can still rename it or change its repo.
        </p>
      ) : (
        <>
          <TargetPicker value={value.target} compact={compact} onChange={pickTarget} />
          {game ? (
            <GameTemplatePicker value={value} onChange={onChange} />
          ) : (
            <>
              <SegmentedControl
                label="How to start"
                options={MODES}
                value={value.mode}
                onChange={(mode: ProjectMode) => onChange({ mode })}
                block
                className={compact ? 'mt-4 w-full' : 'mt-6 w-[320px]'}
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
          label={game ? 'Game repo' : 'Target site repo'}
          size={size}
          placeholder={
            game ? 'The game repo Ship writes models into' : 'Optional, a web repo Ship writes to'
          }
          value={value.siteRepo}
          onChange={siteRepo => onChange({ siteRepo, repo: null, game: null })}
          onCommit={path => void actions.checkRepo(path)}
          onBrowse={() => void actions.browseRepo()}
        />
      </div>
      <RepoLines
        lines={lines}
        label={game ? 'What the game repo holds' : 'What the site repo holds'}
      />
      {game && !locked ? <GameFields value={value} size={size} onChange={onChange} /> : null}
    </>
  );
}
