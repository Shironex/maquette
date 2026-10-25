import { SettingsScreen } from '../SettingsScreen';
import { useSettingsRoot } from './SettingsView.hooks';

/** Remounts per workspace, so a switched workspace never shows the old draft. */
export default function SettingsView() {
  const root = useSettingsRoot();
  return <SettingsScreen key={root} />;
}
