import type { ReactNode } from 'react';
import { StageStepper, TopBar } from '@maquette/ui';
import { useInspectorShell } from './InspectorShell.hooks';

export interface InspectorShellProps {
  children: ReactNode;
  /** The top bar's actions. */
  right?: ReactNode;
}

/** The top bar with the project's stages, over the Inspector's page. */
export default function InspectorShell({ children, right }: InspectorShellProps) {
  const { stages } = useInspectorShell();
  return (
    <>
      <TopBar
        lead={
          stages ? (
            <StageStepper stages={stages} />
          ) : (
            <h1 className="text-panel-title">Inspector</h1>
          )
        }
        right={right}
      />
      {children}
    </>
  );
}
