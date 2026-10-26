import { PathField, TextField, cn } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { ACTIVE } from '../sections';
import { useDevCommand, useProjectSection } from './ProjectSection.hooks';
import type { ProjectSectionProps } from './ProjectSection.types';

export default function ProjectSection({ active }: ProjectSectionProps) {
  const { project, name, setName, repo, setRepo, save, browse } = useProjectSection();
  const dev = useDevCommand(project?.id ?? null, project?.siteRepo ?? null);
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
          label="Target site repo"
          value={repo}
          placeholder="Optional"
          onChange={setRepo}
          onCommit={value => void save(name, value)}
          onBrowse={() => void browse()}
          browseLabel="Browse for the site repo"
          compactBrowse
        />
      </div>
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
    </Panel>
  );
}
