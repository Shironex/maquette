import { SegmentedControl } from '@maquette/ui';
import { clock } from '@/lib/clock';
import { formatAgo } from '@/lib/format';
import { assetCount } from '@/state/board';
import { TEMPLATE_LABEL } from '@/state/onboarding';
import { AddMenu } from './AddMenu';
import { useBoardHeader } from './BoardHeader.hooks';
import { MoreMenu } from './MoreMenu';
import { SendsButton } from './SendsButton';

/** The project name and what it holds, then the Board's controls. */
export default function BoardHeader() {
  const { project, game, layout, setLayout } = useBoardHeader();
  const subtitle = project
    ? `${assetCount((project.assets ?? []).length)} · ${game ? `game · .${game.format} in ${game.modelsDir}/` : TEMPLATE_LABEL[project.template]} · created ${formatAgo(Date.parse(project.createdAt), clock.nowMs())}`
    : 'Create a project in Settings to start.';

  return (
    <div className="flex flex-col gap-3">
      <div className="flex min-w-0 flex-col gap-1">
        <h1 className="text-screen-title">{project?.name ?? 'No project'}</h1>
        <p className="truncate text-fg-mute" title={subtitle}>
          {subtitle}
        </p>
      </div>
      <div className="flex items-center gap-2">
        <SegmentedControl
          label="Board layout"
          options={[
            { value: 'board', label: 'Board', icon: 'squares-four', hint: 'One column per stage' },
            { value: 'list', label: 'List', icon: 'list', hint: 'One sortable row per asset' },
          ]}
          value={layout}
          onChange={setLayout}
        />
        <div className="ml-auto flex items-center gap-2">
          <SendsButton />
          <MoreMenu />
          <AddMenu />
        </div>
      </div>
    </div>
  );
}
