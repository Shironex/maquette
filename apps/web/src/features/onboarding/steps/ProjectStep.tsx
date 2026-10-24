import { useEffect, useState } from 'react';
import { Icon, PathField, SegmentedControl, TextField, cn } from '@maquette/ui';
import { commands, errorMessage, type ImportScan, type ProjectTemplate } from '@/ipc/client';
import diorama from '@/assets/moon-island.jpg';
import room from '@/assets/template-room.jpg';
import character from '@/assets/template-character.jpg';
import { NAME_MAX, nameError, repoNote, type ProjectMode } from '@/state/onboarding';
import type { OnboardingApi } from '../useOnboarding';

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

export function ProjectStep({ api }: { api: OnboardingApi }) {
  const { draft, created } = api;
  const repo = repoNote(draft.repo);
  const [nameTouched, setNameTouched] = useState(false);
  const nameProblem = nameTouched ? nameError(draft.name) : null;

  return (
    <>
      {created ? (
        <p className="mt-7 flex items-center gap-1.5 text-meta text-fg-mute">
          <Icon name="info" size={14} />
          The project exists. You can still rename it or change its site repo.
        </p>
      ) : (
        <>
          <SegmentedControl
            label="How to start"
            options={MODES}
            value={draft.mode}
            onChange={(mode: ProjectMode) => api.update({ mode })}
            block
            className="mt-7 w-[320px]"
          />
          {draft.mode === 'template' ? (
            <TemplateGrid value={draft.template} onChange={template => api.update({ template })} />
          ) : (
            <ImportPicker api={api} />
          )}
        </>
      )}
      <div className="mt-6 grid grid-cols-[minmax(0,0.8fr)_minmax(0,2fr)] gap-3.5">
        <TextField
          label="Project name"
          size="lg"
          value={draft.name}
          maxLength={NAME_MAX}
          onChange={e => api.update({ name: e.target.value })}
          onBlur={() => setNameTouched(true)}
          note={nameProblem ? { tone: 'missing', text: nameProblem } : undefined}
        />
        <PathField
          label="Target site repo"
          size="lg"
          placeholder="Optional, a web repo Ship writes to"
          value={draft.siteRepo}
          onChange={siteRepo => api.update({ siteRepo, repo: null })}
          onCommit={path => void api.checkRepo(path)}
          onBrowse={() => void api.browseRepo()}
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
  onChange,
}: {
  value: ProjectTemplate;
  onChange: (template: Exclude<ProjectTemplate, 'imported'>) => void;
}) {
  return (
    <div className="mt-4 grid grid-cols-3 gap-3.5" role="group" aria-label="Templates">
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
            <div className="relative h-[156px] bg-[#7f8083]">
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
            <div className="px-3 pt-2.5 pb-3">
              <div className="font-semibold">{t.title}</div>
              <div className="mt-0.5 text-meta text-fg-mute">{t.detail}</div>
            </div>
          </button>
        );
      })}
    </div>
  );
}

function ImportPicker({ api }: { api: OnboardingApi }) {
  const folder = api.draft.sourceFolder;
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
      size="lg"
      className="mt-4"
      placeholder="~/Documents/Projects/my-scene"
      value={folder}
      onChange={sourceFolder => api.update({ sourceFolder })}
      onBrowse={() => void api.browseSource()}
      note={note}
    />
  );
}
