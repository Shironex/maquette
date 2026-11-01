import { Button, EmptyState, StageStepper, TopBar, Loader } from '@maquette/ui';
import { AvatarList } from '../AvatarList';
import { AvatarTabs } from '../AvatarTabs';
import { BoneMap } from '../BoneMap';
import { ExportPanel } from '../ExportPanel';
import { MetaForm } from '../MetaForm';
import { PreviewPanel } from '../PreviewPanel';
import { ReadinessPanel } from '../ReadinessPanel';
import { RestPosePanel } from '../RestPosePanel';
import { SpringsPanel } from '../SpringsPanel';
import { useAvatarView } from './AvatarView.hooks';

/**
 * Avatar: a rigged character's VRM 1.0 export. Export checks the rig,
 * fills the meta, shows the rest pose before and after a normalise for a
 * person's yes, and writes `exports/<asset>.vrm`; Springs finds the parts
 * that could swing and chains the confirmed ones for a yes of their own;
 * Preview loads the written file the way a VRM app does.
 */
export default function AvatarView() {
  const { project, stages, count, view, tab, loading, error, retry, building, action, build } =
    useAvatarView();
  const bar = (
    <TopBar
      lead={<StageStepper stages={stages} />}
      right={
        view ? (
          <Button
            variant="primary"
            icon="person-arms-spread"
            loading={building}
            disabled={action.disabled || building}
            tooltip={action.tooltip}
            onClick={build}
          >
            {action.label}
          </Button>
        ) : null
      }
    />
  );
  const empty = !project ? (
    <EmptyState
      icon="person-arms-spread"
      title="No project yet"
      body="Create a project in Settings first."
    />
  ) : count === 0 ? (
    <EmptyState
      icon="person-arms-spread"
      title="No characters yet"
      body="A VRM avatar starts from a rigged character. Characters with a model show here."
    />
  ) : error ? (
    <EmptyState
      icon="warning-circle"
      title="The avatar could not be read"
      body={error}
      action={
        <Button icon="arrow-clockwise" tooltip="Reads the avatar again" onClick={retry}>
          Try again
        </Button>
      }
    />
  ) : null;
  return (
    <>
      {bar}
      <main className="flex min-h-0 flex-1" aria-busy={loading}>
        {count > 0 ? <AvatarList /> : null}
        {empty ? (
          <div className="grid flex-1 place-items-center">{empty}</div>
        ) : view ? (
          <div className="flex min-w-0 flex-1 flex-col overflow-y-auto">
            <div className="mx-auto flex w-full max-w-[1180px] flex-col gap-4 px-6 pt-5 pb-8">
              <AvatarTabs />
              {tab === 'preview' ? (
                <PreviewPanel />
              ) : tab === 'springs' ? (
                <SpringsPanel />
              ) : (
                <>
                  <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
                    <ReadinessPanel />
                    <MetaForm />
                  </div>
                  <RestPosePanel />
                  <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
                    <BoneMap />
                    <ExportPanel />
                  </div>
                </>
              )}
            </div>
          </div>
        ) : (
          <div className="grid flex-1 place-items-center">
            {loading ? <Loader label="Reading the avatar..." /> : null}
          </div>
        )}
      </main>
    </>
  );
}
