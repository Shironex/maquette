import { Button, KeyValueList, Monogram, cn } from '@maquette/ui';
import { Panel } from '@/features/common/Panel';
import { DOCS_URL, REPO_URL, isDevBuild, osName, releaseNotesUrl } from '@/state/about';
import { ACTIVE } from '../sections';
import { UpdatesBlock } from './UpdatesBlock';
import { useAboutSection } from './AboutSection.hooks';
import type { AboutSectionProps } from './AboutSection.types';

export default function AboutSection({ active }: AboutSectionProps) {
  const { about, blender, copied, copy, report, open } = useAboutSection();
  const dev = about ? isDevBuild(about) : false;
  const stamp = about
    ? dev
      ? 'dev build'
      : `commit ${about.commit}, built ${about.buildDate}`
    : 'Reading the build...';

  return (
    <Panel
      id="settings-about"
      title="About"
      help="The version you run, where to read about it, and how to report a problem"
      className={cn('gap-3 px-4 py-3.5', active && ACTIVE)}
    >
      <div className="flex items-center gap-3">
        <Monogram size={36} />
        <div className="min-w-0 flex-1">
          <div className="font-semibold text-fg">
            {about?.name ?? 'Maquette'}{' '}
            <span className="font-mono text-[12px] font-normal">{about?.version}</span>
          </div>
          <div className="truncate font-mono text-[12px] text-fg-mute">{stamp}</div>
        </div>
      </div>
      {about ? (
        <KeyValueList
          columns={2}
          items={[
            { label: 'Platform', value: osName(about) },
            { label: 'Architecture', value: about.arch, mono: true },
            { label: 'Tauri', value: about.tauriVersion, mono: true },
            { label: 'Blender', value: blender ?? 'Not checked yet', mono: Boolean(blender) },
          ]}
        />
      ) : null}
      <div className="h-px bg-line" />
      <UpdatesBlock version={about?.version ?? ''} />
      <div className="h-px bg-line" />
      <div className="flex flex-wrap items-center gap-1.5">
        <Button
          size="sm"
          variant="ghost"
          iconRight="arrow-square-out"
          disabled={!about}
          onClick={() => about && open(releaseNotesUrl(about))}
        >
          Release notes
        </Button>
        <Button
          size="sm"
          variant="ghost"
          iconRight="arrow-square-out"
          onClick={() => open(DOCS_URL)}
        >
          Docs
        </Button>
        <Button
          size="sm"
          variant="ghost"
          iconRight="arrow-square-out"
          onClick={() => open(REPO_URL)}
        >
          Source
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          icon={copied ? 'check' : 'copy'}
          disabled={!about}
          tooltip="Copies the version, OS, Tauri and Blender versions, for a bug report"
          onClick={copy}
        >
          {copied ? 'Copied' : 'Copy version info'}
        </Button>
        <Button size="sm" icon="warning-circle" disabled={!about} onClick={report}>
          Report a problem
        </Button>
      </div>
      <p className="text-meta text-fg-mute">
        Maquette sends no usage data. Report a problem opens the bug form on GitHub in your browser,
        with the version, OS and Blender fields filled in; nothing is sent until you submit it.
      </p>
    </Panel>
  );
}
