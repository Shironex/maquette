import type { WorkspaceConfig } from '@/ipc/client';
import type { ConfigErrors } from '@/state/settings-form';

/** What every config block takes; each block's own props extend it. */
export interface ConfigFieldsProps {
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
  errors?: ConfigErrors;
  /** `roomy` is onboarding, `compact` is the Settings grid. */
  density?: 'roomy' | 'compact';
}
