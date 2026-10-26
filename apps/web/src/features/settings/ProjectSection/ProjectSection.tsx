import { PathField, TextField, cn } from '@maquette/ui';
import { gameOf } from '@/state/game';
import { Panel } from '@/features/common/Panel';
import { ACTIVE } from '../sections';
import { useDevCommand, useModelFolder, useProjectSection } from './ProjectSection.hooks';
import type { ProjectSectionProps } from './ProjectSection.types';

export default function ProjectSection({ active }: ProjectSectionProps) {
  const { project, name, setName, repo, setRepo, save, browse } = useProjectSection();
  const dev = useDevCommand(project?.id ?? null, project?.siteRepo ?? null);
  const game = gameOf(project);
  const folder = useModelFolder(project?.id ?? null, game?.modelsDir ?? '');
  if (!project) return null;

  return (
    <Panel
      id="settings-project"
      title="Project"
      help={`${project.name} only`}
      helpInline
      className={cn('gap-3 px-4 py-3.5 [&>div:first-child]:justify-between', active && ACTIVE)}
    >
      <div className="grid grid-cols-[minmax(0,0.7fr)_minmax(0,1.8fr)] gap-2.5">
        <TextField
          label="Name"
          value={name}
          onChange={e => setName(e.target.value)}
          onBlur={() => void save(name, repo)}
          onKeyDown={e => {
            if (e.key === 'Enter') e.currentTarget.blur();
          }}
        />
        <PathField
          label={game ? 'Game repo' : 'Target site repo'}
          value={repo}
          placeholder="Optional"
          onChange={setRepo}
          onCommit={value => void save(name, value)}
          onBrowse={() => void browse()}
          browseLabel={game ? 'Browse for the game repo' : 'Browse for the site repo'}
          compactBrowse
        />
      </div>
      {game ? (
        <TextField
          label="Model folder"
          value={folder.value}
          mono
          hint={`Models are .${game.format}. Ship writes here and only replaces files it wrote. The Pipeline view sets the export.`}
          onChange={e => folder.setValue(e.target.value)}
          onBlur={() => void folder.save()}
          onKeyDown={e => {
            if (e.key === 'Enter') e.currentTarget.blur();
          }}
        />
      ) : (
        <TextField
          label="Site dev command"
          value={dev.value}
          placeholder={dev.placeholder}
          disabled={!dev.ready}
          mono
          hint="The Page view runs it in the site repo. Leave it empty to use what the site names; {port} is the free port Maquette picks."
          onChange={e => dev.setValue(e.target.value)}
          onBlur={() => void dev.save()}
          onKeyDown={e => {
            if (e.key === 'Enter') e.currentTarget.blur();
          }}
        />
      )}
    </Panel>
  );
}
