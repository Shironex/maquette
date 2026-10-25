import { useCatalog } from '@/features/providers/PresetFields';
import { useAppStore } from '@/state/use-app-store';
import { readySummary } from '@/state/onboarding';
import type { OnboardingApi } from '../../Onboarding';

/** One summary row per setup step. */
export function useReadyRows(api: OnboardingApi) {
  const tools = useAppStore(s => s.tools.reports);
  const catalog = useCatalog();
  return readySummary(api.draft, tools, catalog);
}
