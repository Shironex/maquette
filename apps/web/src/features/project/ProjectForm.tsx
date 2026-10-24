import { useEffect, useState } from 'react';
import { Icon, PathField, SegmentedControl, TextField, cn } from '@maquette/ui';
import { commands, errorMessage, type ImportScan, type ProjectTemplate } from '@/ipc/client';
import diorama from '@/assets/moon-island.jpg';
import room from '@/assets/template-room.jpg';
import character from '@/assets/template-character.jpg';
import {
  NAME_MAX,
  nameError,
  repoNote,
  type ProjectFields,
  type ProjectMode,
} from '@/state/onboarding';
import type { ProjectFieldActions } from './useProjectFieldActions';

const TEMPLATES: readonly {
  id: Exclude<ProjectTemplate, 'imported'>;
  title: string;
  detail: string;
  image: string;
  contain?: boolean;
}[] = [
  {
    id: 'floating-diorama',
    title: 'Floating diorama',
    detail: 'An island scene with camera stops',
    image: diorama,
  },
  { id: 'room', title: 'Room', detail: 'One interior, baked light', image: room },
  {
    id: 'character-showcase',
    title: 'Character showcase',
    detail: 'One rigged model on a turntable',
    image: character,
    contain: true,
  },
];

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
export function ProjectForm({
  value,
  onChange,
  actions,
  locked,
  compact,
  autoFocusName,
}: ProjectFormProps) {
  const repo = repoNote(value.repo);
  const [nameTouched, setNameTouched] = useState(false);
  const nameProblem = nameTouched ? nameError(value.name) : null;
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
          onBlur={() => setNameTouched(true)}
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

function TemplateGrid({
  value,
  compact,
  onChange,
}: {
  value: ProjectTemplate;
  compact?: boolean;
  onChange: (template: Exclude<ProjectTemplate, 'imported'>) => void;
}) {
  return (
    <div
      className={cn('mt-4 grid grid-cols-3', compact ? 'gap-2.5' : 'gap-3.5')}
      role="group"
      aria-label="Templates"
    >
      {TEMPLATES.map(t => {
        const on = t.id === value;
        return (
          <button
            key={t.id}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(t.id)}
            className={cn(
              'flex cursor-pointer flex-col overflow-hidden rounded-card border bg-raised text-left transition-colors duration-120',
              on
                ? 'border-clay shadow-[0_0_0_1px_var(--clay)]'
                : 'border-line hover:border-line-strong'
            )}
          >
            <div className={cn('relative bg-[#7f8083]', compact ? 'h-[88px]' : 'h-[156px]')}>
              <img
                src={t.image}
                alt=""
                className={cn(
                  'block size-full',
                  t.contain ? 'object-contain brightness-[0.84] saturate-[1.04]' : 'object-cover'
                )}
              />
              {on ? (
                <span className="absolute top-2 left-2 inline-flex h-[22px] items-center gap-1.5 rounded-[6px] bg-clay px-2 text-[11.5px] font-medium text-clay-ink">
                  <Icon name="check" size={12} />
                  Selected
                </span>
              ) : null}
            </div>
            <div className={compact ? 'px-2.5 pt-2 pb-2.5' : 'px-3 pt-2.5 pb-3'}>
              <div className={cn('font-semibold', compact && 'text-[12.5px]')}>{t.title}</div>
              {compact ? null : <div className="mt-0.5 text-meta text-fg-mute">{t.detail}</div>}
            </div>
          </button>
        );
      })}
    </div>
  );
}

function ImportPicker({
  folder,
  size,
  onChange,
  onBrowse,
}: {
  folder: string;
  size: 'md' | 'lg';
  onChange: (folder: string) => void;
  onBrowse: () => void;
}) {
  const [scan, setScan] = useState<ImportScan | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setScan(null);
    setError(null);
    if (!folder.trim()) return;
    commands
      .projectScanImport(folder)
      .then(result => live && setScan(result))
      .catch((e: unknown) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [folder]);

  const latest = scan?.blendFiles[0];
  const note = error
    ? { tone: 'missing' as const, text: error }
    : scan
      ? {
          tone: scan.blendFiles.length ? ('found' as const) : ('warning' as const),
          text: scan.blendFiles.length
            ? `${scan.blendFiles.length} .blend files, latest ${latest?.file ?? ''}. ${scan.toolScripts.length} tools/ scripts.`
            : 'No .blend files found. I will import the folder as it is.',
        }
      : undefined;

  return (
    <PathField
      label="Folder to import"
      size={size}
      className="mt-4"
      placeholder="~/Documents/Projects/my-scene"
      value={folder}
      onChange={onChange}
      onBrowse={onBrowse}
      note={note}
    />
  );
}
