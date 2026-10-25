import { PathField, TextField, cn } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { ACTIVE } from '../sections';
import { useProjectSection } from './ProjectSection.hooks';
import type { ProjectSectionProps } from './ProjectSection.types';

export default function ProjectSection({ active }: ProjectSectionProps) {
  const { project, name, setName, repo, setRepo, save, browse } = useProjectSection();
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
    </Panel>
  );
}
