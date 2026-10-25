import type { ConfigFieldsProps } from '../ConfigFieldsProps';

export interface McpControlsProps extends ConfigFieldsProps {
  /** Onboarding has no token yet, so it shows a preview instead. */
  showCopy?: boolean;
}
