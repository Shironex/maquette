import { Button, EmptyState, StageStepper, TopBar, Loader } from '@maquette/ui';
import { ClipStage } from '../ClipStage';
import { ClipsPanel } from '../ClipsPanel';
import { MotionBarActions } from '../MotionBarActions';
import { MotionEmpty } from '../MotionEmpty';
import { MotionList } from '../MotionList';
import { StatesPanel } from '../StatesPanel';
import { StatesStage } from '../StatesStage';
import { useMotionView } from './MotionView.hooks';

/**
 * Motion: the project's characters and props that move. Clips play and
 * scrub in the viewport with what they weigh inside the scene budget;
 * States poses a prop's parts and times the moves between them.
 */
export default function MotionView() {
  const { project, stages, view, loading, error, retry, moving, model, tab } = useMotionView();
  const bar = <TopBar lead={<StageStepper stages={stages} />} right={<MotionBarActions />} />;
  if (!project || !view) {
    return (
      <>
        {bar}
        <main className="grid flex-1 place-items-center" aria-busy={loading}>
          {!project ? (
            <EmptyState
              icon="person-simple-walk"
              title="No project yet"
              body="Create a project in Settings first."
            />
          ) : error ? (
            <EmptyState
              icon="warning-circle"
              title="The models could not be read"
              body={error}
              action={
                <Button icon="arrow-clockwise" tooltip="Reads the models again" onClick={retry}>
                  Try again
                </Button>
              }
            />
          ) : (
            <Loader />
          )}
        </main>
      </>
    );
  }
  return (
    <>
      {bar}
      <main className="flex min-h-0 flex-1">
        <MotionList />
        {moving === 0 && !model ? (
          <MotionEmpty />
        ) : tab === 'states' ? (
          <>
            <StatesStage />
            <StatesPanel />
          </>
        ) : (
          <>
            <ClipStage />
            <ClipsPanel />
          </>
        )}
      </main>
    </>
  );
}
