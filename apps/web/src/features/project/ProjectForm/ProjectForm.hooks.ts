import { useState } from 'react';
import { nameError } from '@/state/onboarding';

/** The name problem shows once the field has been left. */
export function useProjectForm(name: string) {
  const [nameTouched, setNameTouched] = useState(false);
  const nameProblem = nameTouched ? nameError(name) : null;
  const touchName = () => setNameTouched(true);
  return { nameProblem, touchName };
}
